// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

use super::*;
use sp_core::Pair;
use sp_statement_store::{statement_allowance_key, StatementAllowance, StatementSource};
use substrate_test_runtime_client::{DefaultTestClientBuilderExt, TestClientBuilderExt};

#[tokio::test]
async fn run_repairs_both_full_peers_while_live_ticks_are_continuously_ready() {
	let (mut handler, store, _, notifications, _, peers) = build_handler(2);
	enable_full_replication(&mut handler);
	let mut replay_hashes = Vec::new();
	for i in 0..3u8 {
		let mut statement = new_live_statement();
		// Each replay notification can hold only one of these records.
		statement.set_plain_data(vec![i; 600 * 1024]);
		replay_hashes.push(statement.hash());
		store.insert(statement);
	}

	let tick_count = Arc::new(AtomicUsize::new(0));
	let live_hashes = Arc::new(Mutex::new(Vec::new()));
	let (ticks, live) = (tick_count.clone(), live_hashes.clone());
	let busy_ticks = futures::stream::poll_fn(move |_| {
		let tick = ticks.load(Ordering::Relaxed);
		if tick == 32 {
			// Only after the finite busy phase may the test notification service close run().
			return std::task::Poll::Pending;
		}
		// Scheduling uses std::Instant, not Tokio's clock. Give every ready live tick
		// one real burst interval of processing cost so due work is independent of CPU
		// speed. This branch remains Ready throughout; lower select branches cannot run.
		std::thread::sleep(INITIAL_SYNC_BURST_INTERVAL);
		let mut statement = new_live_statement();
		statement.set_plain_data(format!("live tick {tick}").into_bytes());
		let hash = statement.hash();
		store.recent_statements.lock().unwrap().insert(hash, statement);
		live.lock().unwrap().push(hash);
		ticks.fetch_add(1, Ordering::Relaxed);
		std::task::Poll::Ready(Some(()))
	});
	handler.propagate_timeout =
		(Box::pin(busy_ticks) as Pin<Box<dyn Stream<Item = ()> + Send>>).fuse();

	// Exercise the real select loop, completions, live propagation, due-work servicing
	// and peer round-robin scheduling, rather than manually invoking burst processing.
	handler.run().await;
	assert_eq!(tick_count.load(Ordering::Relaxed), 32);
	let sent = notifications.get_sent_notifications();
	let live_hashes = live_hashes.lock().unwrap();
	for peer in peers {
		let hashes = get_peer_hashes(&sent, peer);
		let repaired: Vec<_> =
			hashes.iter().filter(|hash| replay_hashes.contains(hash)).copied().collect();
		assert_eq!(repaired, replay_hashes, "busy run loop must finish replay for each full peer");
		assert!(
			hashes.iter().any(|hash| live_hashes.contains(hash)),
			"live traffic must also progress for each peer"
		);
	}
}

fn native_store(
	client: Arc<substrate_test_runtime_client::TestClient>,
) -> (Arc<sc_statement_store::Store>, tempfile::TempDir) {
	let directory = tempfile::tempdir().unwrap();
	let config = sc_statement_store::Config {
		full_replication: Some(Default::default()),
		..Default::default()
	};
	let store = sc_statement_store::Store::new(
		directory.path(),
		config,
		client,
		Arc::new(sc_keystore::LocalKeystore::in_memory()),
		None,
		Box::new(sp_core::testing::TaskExecutor::new()),
	)
	.unwrap();
	(Arc::new(store), directory)
}

fn signed_at_size(pair: &sp_core::ed25519::Pair, size: usize) -> Statement {
	let mut statement = new_live_statement();
	statement.set_plain_data(vec![0; size]);
	statement.sign_ed25519_private(pair);
	let overhead = statement.encoded_size() - size;
	statement.set_plain_data(vec![0; size - overhead]);
	statement.sign_ed25519_private(pair);
	assert_eq!(statement.encoded_size(), size);
	statement
}

#[tokio::test]
async fn real_store_boundary_round_trips_live_and_replay_over_v1_and_v2() {
	let pair = sp_core::ed25519::Pair::from_seed(&[21; 32]);
	let client = Arc::new(
		substrate_test_runtime_client::TestClientBuilder::new()
			.add_extra_storage(
				statement_allowance_key(pair.public()),
				StatementAllowance::new(16, 2 * 1024 * 1024).encode(),
			)
			.build(),
	);
	let (source, _source_dir) = native_store(client.clone());
	let boundary = signed_at_size(&pair, FULL_REPLICATION_MAX_STATEMENT_SIZE);
	assert_eq!(source.submit(boundary.clone(), StatementSource::Network), SubmitResult::New);
	assert!(matches!(
		source.submit(
			signed_at_size(&pair, FULL_REPLICATION_MAX_STATEMENT_SIZE + 1),
			StatementSource::Network
		),
		SubmitResult::Invalid(sp_statement_store::InvalidReason::EncodingTooLarge { .. })
	));
	assert_eq!(source.statement(&boundary.hash()).unwrap(), Some(boundary.clone()));

	for version in [PeerProtocolVersion::V1, PeerProtocolVersion::V2] {
		let (mut sender, _, _, notifications, _, peers) = build_handler(1);
		enable_full_replication(&mut sender);
		sender.statement_store = source.clone();
		sender.peers.get_mut(&peers[0]).unwrap().protocol_version = version;
		// Live transmission reads the native hash index; replay reads its admission journal.
		sender.do_propagate_statements(&[(0, boundary.hash(), boundary.clone())]);
		sender.flush_pending_sends().await;
		sender.schedule_initial_sync_for_peer(peers[0]);
		sender.process_initial_sync_burst();
		sender.flush_pending_sends().await;
		let sent = notifications.get_sent_notifications();
		assert_eq!(sent.len(), 2, "both native retrieval paths must send the boundary record");

		for (_, bytes) in sent {
			assert!(bytes.len() <= MAX_STATEMENT_NOTIFICATION_SIZE as usize);
			let mut input = bytes.as_slice();
			let records = match version {
				PeerProtocolVersion::V1 => StatementBatch::decode(&mut input).unwrap(),
				PeerProtocolVersion::V2 => match StatementMessage::decode(&mut input).unwrap() {
					StatementMessage::Statements(records) => records,
					_ => panic!("expected Statement payload"),
				},
			};
			assert!(input.is_empty());
			assert_eq!(records, vec![boundary.clone()]);

			let (destination, _destination_dir) = native_store(client.clone());
			let (mut receiver, _, _, _, validation, remote) = build_handler(1);
			enable_full_replication(&mut receiver);
			receiver.statement_store = destination.clone();
			receiver.peers.get_mut(&remote[0]).unwrap().protocol_version = version;
			receiver
				.handle_notification_event(NotificationEvent::NotificationReceived {
					peer: remote[0],
					notification: bytes.into(),
				})
				.await;
			let (statement, completion) = validation
				.try_recv()
				.expect("native wire decoder must enqueue the boundary record");
			assert!(validation.is_empty());
			let result = destination.submit(statement, StatementSource::Network);
			assert_eq!(result, SubmitResult::New);
			completion.send(result).unwrap();
			assert_eq!(destination.statement(&boundary.hash()).unwrap(), Some(boundary.clone()));
			drop(receiver);
			drop(destination);
			drop(_destination_dir);
		}
	}
	drop(source);
	drop(_source_dir);
}
