// Copyright 2018 Commonwealth Labs, Inc.
// This file is part of Edgeware.

// Edgeware is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Edgeware is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Edgeware.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate serde;

// Needed for deriving `Serialize` and `Deserialize` for various types.
// We only implement the serde traits for std builds - they're unneeded
// in the wasm runtime.
#[cfg(feature = "std")]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate hex_literal;
#[macro_use] extern crate parity_codec_derive;
#[macro_use] extern crate srml_support;


extern crate parity_codec as codec;
extern crate substrate_primitives as primitives;
#[cfg_attr(not(feature = "std"), macro_use)]
extern crate sr_std as rstd;
extern crate srml_support as runtime_support;
extern crate sr_primitives as runtime_primitives;
extern crate sr_io as runtime_io;

extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate edge_delegation as delegation;

pub mod voting;
pub use voting::{Module, Trait, RawEvent, Event};
pub use voting::{VoteStage, VoteType, TallyType, VoteRecord, VoteData, VoteOutcome};

pub static YES_VOTE: VoteOutcome = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1];
pub static NO_VOTE: VoteOutcome = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

// Tests for Delegation Module
#[cfg(test)]
mod tests {
	use super::*;
	use rstd::prelude::*;
	use runtime_support::dispatch::Result;
	use runtime_io::ed25519::Pair;
	use system::{EventRecord, Phase};
	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use rstd::result;
	// The testing primitives are very useful for avoiding having to work with signatures
	// or public keys. `u64` is used as the `AccountId` and no `Signature`s are requried.
	use runtime_primitives::{
		BuildStorage, traits::{BlakeTwo256, Hash}, testing::{Digest, DigestItem, Header}
	};

	static SECRET: [u8; 32] = [1,0,1,0,1,0,1,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4];

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	impl_outer_event! {
		pub enum Event for Test {
			voting<T>, delegation<T>, balances<T>,
		}
	}

	impl_outer_dispatch! {
		pub enum Call for Test where origin: Origin {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = H256;
		type Header = Header;
		type Event = Event;
		type Log = DigestItem;
	}

	impl balances::Trait for Test {
		type Balance = u64;
		type AccountIndex = u64;
		type OnFreeBalanceZero = ();
		type EnsureAccountLiquid = ();
		type Event = Event;
	}

	impl delegation::Trait for Test {
		type Event = Event;
	}

	impl Trait for Test {
		type Event = Event;
	}

	pub type System = system::Module<Test>;
	pub type Delegation = delegation::Module<Test>;
	pub type Voting = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
		let t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		// We use default for brevity, but you can configure as desired if needed.
		t.into()
	}

	fn create_vote(
		who: H256,
		vote_type: voting::VoteType,
		is_commit_reveal: bool,
		tally_type: voting::TallyType,
		outcomes: &[[u8; 32]]
	) -> result::Result<u64, &'static str> {
		Voting::create_vote(who,
							vote_type,
							is_commit_reveal,
							tally_type,
							outcomes.to_vec())
	}

	fn commit(who: H256, vote_id: u64, commit: [u8; 32]) -> Result {
		Voting::commit(Origin::signed(who), vote_id, commit)
	}

	fn reveal(who: H256, vote_id: u64, vote: [u8; 32], secret: Option<[u8; 32]>) -> Result {
		Voting::reveal(Origin::signed(who), vote_id, vote, secret)
	}

	fn advance_stage_as_initiator(who: H256, vote_id: u64) -> Result {
		Voting::advance_stage_as_initiator(Origin::signed(who), vote_id)
	}

	fn delegate_to(who: H256, to: H256) -> Result {
		Delegation::delegate_to(Origin::signed(who), to)
	}

	fn get_test_key() -> H256 {
		let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
		let public: H256 = pair.public().0.into();
		return public;
	}

	fn get_test_key_2() -> H256 {
		let pair: Pair = Pair::from_seed(&hex!("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"));
		let public: H256 = pair.public().0.into();
		return public;		
	}

	fn generate_1p1v_public_binary_vote() -> (voting::VoteType, bool, voting::TallyType, [[u8; 32]; 2]) {
		let vote_type = VoteType::Binary;
		let tally_type = TallyType::OnePerson;
		let is_commit_reveal = false;
		let yes_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1];
		let no_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

		return (vote_type, is_commit_reveal, tally_type, [yes_outcome, no_outcome]);
	}

	fn generate_1p1v_commit_reveal_binary_vote() -> (voting::VoteType, bool, voting::TallyType, [[u8; 32]; 2]) {
		let vote_type = VoteType::Binary;
		let tally_type = TallyType::OnePerson;
		let is_commit_reveal = true;
		let yes_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1];
		let no_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];

		return (vote_type, is_commit_reveal, tally_type, [yes_outcome, no_outcome]);
	}

	fn generate_1p1v_public_multi_vote() -> (voting::VoteType, bool, voting::TallyType, [[u8; 32]; 4]) {
		let vote_type = VoteType::MultiOption;
		let tally_type = TallyType::OnePerson;
		let is_commit_reveal = false;
		let one_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1];
		let two_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2];
		let three_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3];
		let four_outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4];

		return (vote_type, is_commit_reveal, tally_type, [
			one_outcome,
			two_outcome,
			three_outcome,
			four_outcome
		]);
	}

	fn make_record(
		id: u64,
		author: H256,
		vote_type: voting::VoteType,
		is_commit_reveal: bool,
		tally_type: voting::TallyType,
		outcomes: &[[u8; 32]],
		stage: VoteStage
	) -> VoteRecord<H256> {
		VoteRecord {
			id: id,
			commitments: vec![],
			reveals: vec![],
			outcomes: outcomes.to_vec(),
			data: VoteData {
				initiator: author,
				stage: stage,
				vote_type: vote_type,
				tally_type: tally_type,
				is_commit_reveal: is_commit_reveal,
			},
		}
	}

	#[test]
	fn create_binary_vote_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_eq!(Voting::vote_record_count(), 1);
			assert_eq!(
				Voting::vote_records(1),
				Some(make_record(1, public, vote.0, vote.1, vote.2, &vote.3, VoteStage::PreVoting))
			);
			assert_eq!(System::events(), vec![
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCreated(1, public, VoteType::Binary))
				}
			]);
		});
	}

	#[test]
	fn create_binary_vote_with_multi_options_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			let multi_vote = generate_1p1v_public_multi_vote();
			assert_err!(create_vote(public, vote.0, vote.1, vote.2, &multi_vote.3), "Invalid binary outcomes");
			assert_eq!(Voting::vote_record_count(), 0);
			assert_eq!(Voting::vote_records(1), None);
		});
	}

	#[test]
	fn create_multi_vote_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_multi_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_eq!(Voting::vote_record_count(), 1);
			assert_eq!(
				Voting::vote_records(1),
				Some(make_record(1, public, vote.0, vote.1, vote.2, &vote.3, VoteStage::PreVoting))
			);
		});
	}

	#[test]
	fn create_multi_vote_with_binary_options_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			let multi_vote = generate_1p1v_public_multi_vote();
			assert_err!(create_vote(public, multi_vote.0, multi_vote.1, multi_vote.2, &vote.3), "Invalid multi option outcomes");
			assert_eq!(Voting::vote_record_count(), 0);
			assert_eq!(Voting::vote_records(1), None);
		});
	}

	#[test]
	fn create_vote_with_one_outcome_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_multi_vote();
			let outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4];
			assert_err!(create_vote(public, vote.0, vote.1, vote.2, &[outcome]), "Invalid multi option outcomes");
			assert_eq!(Voting::vote_record_count(), 0);
			assert_eq!(Voting::vote_records(1), None);
		});
	}

	// TODO: Ensure we fix this test when we support these types!
	#[test]
	fn create_vote_with_unsupported_type_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_multi_vote();
			let outcome: [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4];
			assert_err!(create_vote(public, VoteType::AnonymousRing, vote.1, vote.2, &[outcome]), "Unsupported vote type");
			assert_eq!(Voting::vote_record_count(), 0);
			assert_eq!(Voting::vote_records(1), None);
		});
	}

	#[test]
	fn commit_to_nonexistent_record_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let commit_value = SECRET;
			assert_err!(commit(public, 1, commit_value), "Vote record does not exist");
		});
	}

	#[test]
	fn commit_to_non_commit_record_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			let commit_value = SECRET;
			assert_err!(commit(public, 1, commit_value), "Commitments are not configured for this vote");
		});
	}

	#[test]
	fn reveal_to_nonexistent_record_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let commit_value = SECRET;
			assert_err!(reveal(public, 1, commit_value, Some(commit_value)), "Vote record does not exist");
		});
	}

	#[test]
	fn reveal_to_record_before_voting_period_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			let vote_outcome = vote.3[0];
			assert_err!(reveal(public, 1, vote_outcome, Some(vote_outcome)), "Vote is not in voting stage");
		});
	}

	#[test]
	fn advance_from_non_initiator_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let public2 = get_test_key_2();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_err!(advance_stage_as_initiator(public2, 1), "Invalid advance attempt by non-owner");
		});
	}

	#[test]
	fn advance_from_initiator_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(public, 1));
			assert_eq!(
				Voting::vote_records(1),
				Some(make_record(1, public, vote.0, vote.1, vote.2, &vote.3, VoteStage::Voting))
			);
			assert_eq!(System::events(), vec![
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCreated(1, public, VoteType::Binary))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::PreVoting, VoteStage::Voting))
				}
			]);
		});
	}

	#[test]
	fn reveal_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(public, 1));
			let public2 = get_test_key_2();
			assert_ok!(reveal(public2, 1, vote.3[0], Some(vote.3[0])));
			assert_eq!(
				Voting::vote_records(1).unwrap().reveals,
				vec![(public2, vote.3[0])]
			);
			assert_eq!(System::events(), vec![
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCreated(1, public, VoteType::Binary))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::PreVoting, VoteStage::Voting))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteRevealed(1, public2, vote.3[0]))
				}
			]);
		});
	}

	#[test]
	fn complete_after_reveal_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(public, 1));
			let public2 = get_test_key_2();
			assert_ok!(reveal(public2, 1, vote.3[0], Some(vote.3[0])));
			assert_ok!(advance_stage_as_initiator(public, 1));
			assert_eq!(
				Voting::vote_records(1).unwrap().data.stage,
				VoteStage::Completed
			);
			assert_eq!(System::events(), vec![
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCreated(1, public, VoteType::Binary))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::PreVoting, VoteStage::Voting))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteRevealed(1, public2, vote.3[0]))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::Voting, VoteStage::Completed))
				}
			]);
		});
	}

	#[test]
	fn transition_to_commit_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_commit_reveal_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_eq!(
				Voting::vote_records(1).unwrap().data.is_commit_reveal,
				true
			);
			assert_ok!(advance_stage_as_initiator(public, 1));
			assert_eq!(
				Voting::vote_records(1).unwrap().data.stage,
				VoteStage::Commit
			);
			assert_eq!(System::events(), vec![
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCreated(1, public, VoteType::Binary))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::PreVoting, VoteStage::Commit))
				}
			]);
		});
	}

	#[test]
	fn reveal_before_commit_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_commit_reveal_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_eq!(
				Voting::vote_records(1).unwrap().data.is_commit_reveal,
				true
			);
			let public2 = get_test_key_2();
			assert_err!(reveal(public2, 1, vote.3[0], Some(vote.3[0])), "Vote is not in voting stage");
		});
	}

	#[test]
	fn reveal_commit_before_stage_change_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_commit_reveal_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(public, 1));
			let public2 = get_test_key_2();
			let secret = SECRET;
			let mut buf = Vec::new();
			buf.extend_from_slice(&<[u8; 32]>::from(public2));
			buf.extend_from_slice(&secret);
			buf.extend_from_slice(&vote.3[0]);
			let commit_hash: [u8; 32] = BlakeTwo256::hash_of(&buf).into();
			assert_ok!(commit(public2, 1, commit_hash));
			assert_eq!(
				Voting::vote_records(1).unwrap().commitments,
				vec![(public2, commit_hash)]
			);

			assert_err!(reveal(public2, 1, vote.3[0], Some(secret)), "Vote is not in voting stage");
		});
	}

	#[test]
	fn reveal_commit_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_commit_reveal_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(public, 1));
			let public2 = get_test_key_2();
			let secret = SECRET;
			let mut buf = Vec::new();
			buf.extend_from_slice(&<[u8; 32]>::from(public2));
			buf.extend_from_slice(&secret);
			buf.extend_from_slice(&vote.3[0]);
			let commit_hash: [u8; 32] = BlakeTwo256::hash_of(&buf).into();
			assert_ok!(commit(public2, 1, commit_hash));
			assert_eq!(
				Voting::vote_records(1).unwrap().commitments,
				vec![(public2, commit_hash)]
			);

			assert_ok!(advance_stage_as_initiator(public, 1));
			assert_ok!(reveal(public2, 1, vote.3[0], Some(secret)));
			assert_eq!(System::events(), vec![
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCreated(1, public, VoteType::Binary))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::PreVoting, VoteStage::Commit))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteCommitted(1, public2))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteAdvanced(1, VoteStage::Commit, VoteStage::Voting))
				},
				EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::voting(voting::RawEvent::VoteRevealed(1, public2, vote.3[0]))
				}
			]);
		});
	}

	#[test]
	fn tally_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);
			let public = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(public, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(public, 1));
			assert_ok!(reveal(public, 1, vote.3[0], Some(vote.3[0])));
			assert_ok!(advance_stage_as_initiator(public, 1));
			assert_eq!(
				Voting::tally(1).unwrap(),
				vec![(vote.3[0], 1), (vote.3[1], 0)]
			);
		});
	}

	#[test]
	fn delegation_should_work() {
		with_externalities(&mut new_test_ext(), || {
			/*  To test delegation, we'll generate a delegation graph, have some
			 *  users vote, then make sure the vote tallies as expected.
			 *  Delegation graph:
			 *    1 --> 2 --> 3 --> 4
			 *                ^     ^
			 *                |     |
			 *                5     6
			 *  Voters: 2 (0x0), 4 (0x1), 5 (0x0)
			 *  Expected Tally: 3 votes for 0x0, 3 votes for 0x1
			 */
			System::set_block_number(1);
			// set up delegations
			let users : Vec<H256> = (0..7).map(|v| H256::from(v)).collect();
			assert_ok!(delegate_to(users[1], users[2]));
			assert_ok!(delegate_to(users[2], users[3]));
			assert_ok!(delegate_to(users[3], users[4]));
			assert_ok!(delegate_to(users[5], users[3]));
			assert_ok!(delegate_to(users[6], users[4]));

			let creator = get_test_key();
			let vote = generate_1p1v_public_binary_vote();
			assert_eq!(Ok(1), create_vote(creator, vote.0, vote.1, vote.2, &vote.3));
			assert_ok!(advance_stage_as_initiator(creator, 1));

			// perform votes
			assert_ok!(reveal(users[2], 1, vote.3[0], None));
			assert_ok!(reveal(users[4], 1, vote.3[1], None));
			assert_ok!(reveal(users[5], 1, vote.3[0], None));
			assert_ok!(advance_stage_as_initiator(creator, 1));
			assert_eq!(
				Voting::tally(1).unwrap(),
				vec![(vote.3[0], 3), (vote.3[1], 3)]
			);
		});
	}
}
