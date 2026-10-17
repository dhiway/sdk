// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;
use crate::{Config, Eviction};
use sc_network_statement::config::{FullReplication, FULL_REPLICATION_MAX_STATEMENT_SIZE};
use std::sync::{Arc, Barrier};

fn full_store() -> (Store, tempfile::TempDir) {
	let (mut store, temp) = test_store();
	store.set_time(0);
	store.submit_index.write().config.full_replication = Some(FullReplication::default());
	(store, temp)
}

fn reopen(temp: &tempfile::TempDir, config: Config) -> crate::Result<Store> {
	Store::new::<Block, TestClient, TestBackend>(
		&temp.path().join("db"),
		config,
		Arc::new(TestClient),
		Arc::new(sc_keystore::LocalKeystore::in_memory()),
		None,
		Box::new(sp_core::testing::TaskExecutor::new()),
	)
}

fn retained(store: &Store, statements: &[Statement]) -> Vec<sp_statement_store::Hash> {
	let mut hashes: Vec<_> = statements
		.iter()
		.filter(|s| store.has_statement(&s.hash()))
		.map(Statement::hash)
		.collect();
	hashes.sort();
	hashes
}

fn assert_consistent(store: &Store) {
	// Hash-column raw iteration omits ParityDB's commit overlay. Ordered indexes and
	// point reads reflect the same committed view used by live admission and queries.
	let statements = store.statements().unwrap();
	assert_eq!(store.statement_count(), statements.len());
	assert_eq!(store.total_size(), statements.iter().map(|(_, s)| s.data_len()).sum::<usize>());
	let mut bans = std::collections::HashSet::new();
	let mut iter = store.db.iter(col::INDEX_EVICTED).unwrap();
	iter.seek_to_first().unwrap();
	while let Some((key, _)) = iter.next().unwrap() {
		let (_, hash) = crate::parse_time_index_key(&key).unwrap();
		if store.db.get(col::EXPIRED, &hash).unwrap().is_some() {
			bans.insert(hash);
		}
	}
	assert_eq!(store.evicted_count(), bans.len());
	let mut admitted = 0;
	let mut iter = store.db.iter(col::ADMISSION_SEQ).unwrap();
	iter.seek_to_first().unwrap();
	while let Some((_, hash)) = iter.next().unwrap() {
		assert!(store.has_statement(&hash.try_into().unwrap()));
		admitted += 1;
	}
	assert_eq!(admitted, statements.len());
}

#[test]
fn channel_and_count_ties_choose_the_same_hash_in_opposite_orders() {
	for channel in [None, Some(1)] {
		let mut statements = [statement(1, 5, channel, 100), statement(1, 5, channel, 101)];
		statements.sort_by_key(Statement::hash);
		let (a, _ta) = full_store();
		let (b, _tb) = full_store();
		for s in &statements {
			a.submit(s.clone(), StatementSource::Network);
		}
		for s in statements.iter().rev() {
			b.submit(s.clone(), StatementSource::Network);
		}
		assert_eq!(retained(&a, &statements), vec![statements[1].hash()]);
		assert_eq!(retained(&a, &statements), retained(&b, &statements));
		assert_eq!(a.evicted_count(), 0);
		assert_eq!(b.evicted_count(), 0);
		assert_consistent(&a);
		assert_consistent(&b);
	}
}

#[test]
fn unequal_size_competition_recovers_through_repeated_exchange() {
	let statements =
		[statement(2, 5, None, 200), statement(2, 5, None, 500), statement(2, 5, None, 700)];
	for rotation in 0..statements.len() {
		let (a, _ta) = full_store();
		let (b, _tb) = full_store();
		for i in 0..statements.len() {
			a.submit(statements[(i + rotation) % 3].clone(), StatementSource::Network);
		}
		for i in (0..statements.len()).rev() {
			b.submit(statements[(i + rotation) % 3].clone(), StatementSource::Network);
		}
		for _ in 0..4 {
			let survivors: Vec<_> = statements
				.iter()
				.filter(|s| a.has_statement(&s.hash()) || b.has_statement(&s.hash()))
				.cloned()
				.collect();
			for s in survivors {
				a.submit(s.clone(), StatementSource::Network);
				b.submit(s, StatementSource::Network);
			}
		}
		assert_eq!(retained(&a, &statements), retained(&b, &statements));
		assert!(a.statement_count() <= 2);
		assert!(a.total_size() <= 1000);
		assert_eq!(a.evicted_count(), 0);
		assert_consistent(&a);
		assert_consistent(&b);
	}
}

#[test]
fn automatic_allowance_eviction_can_be_repaired_after_restoration() {
	let (mut store, _temp) = full_store();
	let s = statement(1, 5, None, 100);
	assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::New);
	store.read_allowance_fn = Box::new(|_, _| Ok(None));
	store.enforce_limits();
	assert!(!store.has_statement(&s.hash()));
	assert!(!store.is_evicted(&s.hash()));
	store.read_allowance_fn = Box::new(|_, _| Ok(Some(StatementAllowance::new(1, 1000))));
	assert_eq!(store.submit(s, StatementSource::Network), SubmitResult::New);
	assert_consistent(&store);
}

#[test]
fn explicit_suppression_survives_restart_and_authorized_override() {
	let (mut store, temp) = full_store();
	store.time_override = None;
	let s = statement(1, 5, None, 100);
	assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::New);
	store.remove_by(account(1)).unwrap();
	assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::KnownExpired);
	let config = store.submit_index.read().config;
	drop(store);
	let store = reopen(&temp, config).unwrap();
	assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::KnownExpired);
	assert_eq!(store.submit(s.clone(), StatementSource::Local), SubmitResult::New);
	assert_eq!(store.evicted_count(), 0);
	store.remove(&s.hash()).unwrap();
	assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::KnownExpired);
	assert_eq!(store.submit(s, StatementSource::Chain), SubmitResult::New);
	assert_consistent(&store);
}

#[test]
fn classified_automatic_suppression_is_reconsidered_on_activation() {
	let (store, temp) = test_store();
	let low = statement(1, 1, None, 100);
	let high = statement(1, 2, None, 100);
	store.submit(low.clone(), StatementSource::Network);
	store.submit(high.clone(), StatementSource::Network);
	assert!(store.is_evicted(&low.hash()));
	let mut config = store.submit_index.read().config;
	config.full_replication = Some(FullReplication::default());
	drop(store);
	let mut store = reopen(&temp, config).unwrap();
	store.read_allowance_fn = Box::new(|_, _| Ok(Some(StatementAllowance::new(2, 1000))));
	assert_eq!(store.submit(low, StatementSource::Network), SubmitResult::New);
	assert_consistent(&store);
}

#[test]
fn legacy_suppression_blocks_activation_without_being_erased() {
	let (store, temp) = test_store();
	let s = statement(1, 1, None, 100);
	let now = store.timestamp().saturating_sub(10);
	let purge_at = now + store.submit_index.read().config.purge_after_sec;
	store
		.db
		.commit([
			(col::EXPIRED, s.hash().to_vec(), Some((s.hash(), now).encode())),
			(col::INDEX_EVICTED, crate::evicted_index_key(purge_at, &s.hash()), Some(Vec::new())),
		])
		.unwrap();
	let mut config = store.submit_index.read().config;
	config.full_replication = Some(FullReplication::default());
	// Today's purge period cannot redefine a legacy row's existing journal deadline.
	config.purge_after_sec = 1;
	drop(store);
	assert!(
		matches!(reopen(&temp, config), Err(crate::Error::InvalidConfig(message)) if message.contains("legacy suppression"))
	);
	config.full_replication = None;
	let mut store = reopen(&temp, config).unwrap();
	assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::KnownExpired);
	assert_consistent(&store);
	store.set_time(purge_at);
	store.maintain();
	assert!(!store.is_evicted(&s.hash()));
	config.full_replication = Some(FullReplication::default());
	drop(store);
	assert!(reopen(&temp, config).is_ok());
}

#[test]
fn legacy_stale_purge_cannot_erase_a_newer_legacy_ban_before_activation() {
	// Reproduce baseline-generated state: remove at t=0 (deadline 10), authorized
	// re-admit then remove at t=5 (deadline 15), leaving both empty journal rows.
	// Opening in full mode at t=10 must reject activation without purging either row.
	for version in [crate::CURRENT_VERSION, 1] {
		let (store, temp) = test_store();
		let hash = statement(1, 1, None, 100).hash();
		let t10 = store.timestamp();
		let newer_legacy_ban = (hash, t10 - 5).encode();
		let old_journal = crate::evicted_index_key(t10, &hash);
		let new_journal = crate::evicted_index_key(t10 + 5, &hash);
		store
			.db
			.commit([
				(col::META, KEY_VERSION.to_vec(), Some(version.to_le_bytes().to_vec())),
				(col::EXPIRED, hash.to_vec(), Some(newer_legacy_ban.clone())),
				(col::INDEX_EVICTED, old_journal.clone(), Some(Vec::new())),
				(col::INDEX_EVICTED, new_journal.clone(), Some(Vec::new())),
			])
			.unwrap();
		let mut config = store.submit_index.read().config;
		config.purge_after_sec = 10;
		config.full_replication = Some(FullReplication::default());
		drop(store);
		assert!(matches!(reopen(&temp, config), Err(crate::Error::InvalidConfig(_))));
		// Inspect directly: reopening in default mode would invoke the historical purge
		// behavior that this full-mode preflight must never run ahead of its safety gate.
		let db = Store::open_db(&temp.path().join("db/statements")).unwrap();
		assert_eq!(db.get(col::EXPIRED, &hash).unwrap(), Some(newer_legacy_ban));
		assert_eq!(db.get(col::INDEX_EVICTED, &old_journal).unwrap(), Some(Vec::new()));
		assert_eq!(db.get(col::INDEX_EVICTED, &new_journal).unwrap(), Some(Vec::new()));
		assert_eq!(db.get(col::META, KEY_VERSION).unwrap(), Some(version.to_le_bytes().to_vec()));
	}
}

#[test]
fn legacy_rejection_precedes_missing_version_initialization() {
	let (store, temp) = test_store();
	let hash = statement(1, 1, None, 100).hash();
	store
		.db
		.commit([
			(col::META, KEY_VERSION.to_vec(), None),
			(col::EXPIRED, hash.to_vec(), Some((hash, store.timestamp()).encode())),
		])
		.unwrap();
	let mut config = store.submit_index.read().config;
	config.full_replication = Some(Default::default());
	drop(store);
	assert!(matches!(reopen(&temp, config), Err(crate::Error::InvalidConfig(_))));
	let db = Store::open_db(&temp.path().join("db/statements")).unwrap();
	assert_eq!(db.get(col::META, KEY_VERSION).unwrap(), None);
	assert!(db.get(col::EXPIRED, &hash).unwrap().is_some());
}

#[test]
fn malformed_layout_is_rejected_without_adding_missing_authoritative_columns() {
	let temp = tempfile::tempdir().unwrap();
	let path = temp.path().join("db/statements");
	let options = parity_db::Options::with_columns(&path, 2);
	{
		let db = parity_db::Db::open_or_create(&options).unwrap();
		db.commit([(col::META, KEY_VERSION.to_vec(), Some(1u32.to_le_bytes().to_vec()))])
			.unwrap();
	}
	let before = std::fs::read(path.join("metadata")).unwrap();
	let config = Config { full_replication: Some(Default::default()), ..Default::default() };
	assert!(matches!(reopen(&temp, config), Err(crate::Error::Db(_))));
	assert_eq!(std::fs::read(path.join("metadata")).unwrap(), before);
	let db = parity_db::Db::open(&options).unwrap();
	assert_eq!(db.get(col::META, KEY_VERSION).unwrap(), Some(1u32.to_le_bytes().to_vec()));
}

#[test]
fn stale_legacy_and_tagged_purges_cannot_erase_a_new_explicit_ban() {
	for legacy in [false, true] {
		let (mut store, _temp) = full_store();
		store.submit_index.write().config.purge_after_sec = 10;
		let s = statement(1, 1, None, 100);
		let hash = s.hash();
		store.submit(s.clone(), StatementSource::Network);
		store.remove(&hash).unwrap();
		let old = store.db.get(col::EXPIRED, &hash).unwrap().unwrap();
		store.submit(s.clone(), StatementSource::Local);
		store.set_time(5);
		store.remove(&hash).unwrap();
		store
			.db
			.commit([(
				col::INDEX_EVICTED,
				crate::evicted_index_key(10, &hash),
				Some(if legacy { Vec::new() } else { old }),
			)])
			.unwrap();
		store.set_time(10);
		store.maintain();
		assert_eq!(store.submit(s.clone(), StatementSource::Network), SubmitResult::KnownExpired);
		assert_eq!(
			Eviction::decode(&store.db.get(col::EXPIRED, &hash).unwrap().unwrap(), 10)
				.unwrap()
				.purge_at,
			15
		);
		assert_consistent(&store);
		store.set_time(15);
		store.maintain();
		assert_eq!(store.submit(s, StatementSource::Network), SubmitResult::New);
		assert_consistent(&store);
	}
}

#[test]
fn removal_between_validation_and_admission_still_suppresses_network_replay() {
	let (mut store, _temp) = full_store();
	let validated = Arc::new(Barrier::new(2));
	let resume = Arc::new(Barrier::new(2));
	let (v, r) = (validated.clone(), resume.clone());
	store.read_allowance_fn = Box::new(move |_, _| {
		v.wait();
		r.wait();
		Ok(Some(StatementAllowance::new(1, 1000)))
	});
	let store = Arc::new(store);
	let s = statement(1, 1, None, 100);
	let (worker_store, candidate) = (store.clone(), s.clone());
	let worker =
		std::thread::spawn(move || worker_store.submit(candidate, StatementSource::Network));
	validated.wait();
	store.force_insert(&s);
	store.remove(&s.hash()).unwrap();
	resume.wait();
	assert_eq!(worker.join().unwrap(), SubmitResult::KnownExpired);
	assert!(!store.has_statement(&s.hash()));
	assert_consistent(&store);
}

fn statement_at_size(size: usize) -> Statement {
	let mut s = statement(42, 1, None, size - 512);
	let data_len = s.data_len() + size - s.encoded_size();
	s.set_plain_data(vec![0; data_len]);
	sign_with(&mut s, 42);
	assert_eq!(s.encoded_size(), size);
	s
}

#[test]
fn full_mode_encoded_boundary_and_existing_oversize_activation() {
	let limit = FULL_REPLICATION_MAX_STATEMENT_SIZE;
	let (store, _temp) = full_store();
	assert_eq!(store.submit(statement_at_size(limit), StatementSource::Network), SubmitResult::New);
	assert!(
		matches!(store.submit(statement_at_size(limit + 1), StatementSource::Network), SubmitResult::Invalid(InvalidReason::EncodingTooLarge { max_size, .. }) if max_size == limit)
	);
	let (store, temp) = test_store();
	assert_eq!(
		store.submit(statement_at_size(limit + 1), StatementSource::Network),
		SubmitResult::New
	);
	let mut config = store.submit_index.read().config;
	config.full_replication = Some(FullReplication::default());
	drop(store);
	assert!(
		matches!(reopen(&temp, config), Err(crate::Error::InvalidConfig(message)) if message.contains("wire-safe limit"))
	);
	config.full_replication = None;
	let store = reopen(&temp, config).unwrap();
	assert_eq!(store.statement_count(), 1);
	store.remove(&statement_at_size(limit + 1).hash()).unwrap();
	config.full_replication = Some(FullReplication::default());
	drop(store);
	assert!(reopen(&temp, config).is_ok());
}

#[test]
fn full_config_and_burst_capacity_are_validated() {
	let mut config = Config::default();
	assert!(config.full_replication.is_none());
	config.full_replication = Some(FullReplication { repair_interval: std::time::Duration::ZERO });
	assert!(config.validate().is_err());
	config.full_replication = Some(FullReplication::default());
	config.rate_limit = u32::MAX;
	assert!(config.validate().is_err());
	config.rate_limit = 0;
	assert!(config.validate().is_ok());
}
