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
#[macro_use]
extern crate parity_codec_derive;
#[macro_use]
extern crate srml_support;

extern crate parity_codec as codec;
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
extern crate sr_std as rstd;
extern crate srml_support as runtime_support;
extern crate substrate_primitives as primitives;

extern crate srml_system as system;
extern crate srml_timestamp as timestamp;
extern crate srml_consensus as consensus;

pub mod identity;
pub use identity::{
	Event, Module, RawEvent, Trait,
	IdentityStage, MetadataRecord, IdentityRecord
};

// Tests for Identity Module
#[cfg(test)]
mod tests {
	use super::*;

	use codec::Encode;
	use primitives::{Blake2Hasher, H256, Hasher};
	use rstd::prelude::*;
	use runtime_io::ed25519::Pair;
	use runtime_io::with_externalities;
	use runtime_support::dispatch::Result;
	use system::{EventRecord, Phase};
	// The testing primitives are very useful for avoiding having to work with
	// public keys. `u64` is used as the `AccountId` and no `Signature`s are requried.
	use runtime_primitives::{
		testing::{Digest, DigestItem, Header, UintAuthorityId},
		traits::{BlakeTwo256, OnFinalise, IdentityLookup},
		BuildStorage,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	impl_outer_event! {
		pub enum Event for Test {
			identity<T>,
		}
	}

	impl_outer_dispatch! {
		pub enum Call for Test where origin: Origin { }
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
		type Lookup = IdentityLookup<H256>;
		type Header = Header;
		type Event = Event;
		type Log = DigestItem;
	}
	impl consensus::Trait for Test {
		type Log = DigestItem;
		type SessionKey = UintAuthorityId;
		type InherentOfflineReport = ();
	}
	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
	}
	impl Trait for Test {
		type Event = Event;
	}

	type System = system::Module<Test>;
 	type Timestamp = timestamp::Module<Test>;
	type Identity = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		// We use default for brevity, but you can configure as desired if needed.
		t.extend(
			identity::GenesisConfig::<Test> {
				expiration_time: 10000,
				verifiers: [H256::from_low_u64_be(1)].to_vec(),
			}.build_storage().unwrap().0,
		);
		t.into()
	}

	fn register_identity(who: H256, identity_type: &[u8], identity: &[u8]) -> Result {
		Identity::register(Origin::signed(who), identity_type.to_vec(), identity.to_vec())
	}

	fn attest_to_identity(who: H256, identity_hash: H256, attestation: &[u8]) -> Result {
		Identity::attest(Origin::signed(who), identity_hash, attestation.to_vec())
	}

	fn verify_identity(who: H256, identity_hash: H256, approve: bool, verifier_index: usize) -> Result {
		Identity::verify_or_deny(Origin::signed(who), identity_hash, approve, verifier_index)
	}

	fn add_metadata_to_account(
		who: H256,
		identity_hash: H256,
		avatar: &[u8],
		display_name: &[u8],
		tagline: &[u8],
	) -> Result {
		Identity::add_metadata(
			Origin::signed(who),
			identity_hash,
			avatar.to_vec(),
			display_name.to_vec(),
			tagline.to_vec(),
		)
	}

	fn default_identity_record(public: H256, identity_type: &[u8], identity: &[u8]) -> IdentityRecord<H256, u64> {
		IdentityRecord {
			account: public,
			identity_type: identity_type.to_vec(),
			identity: identity.to_vec(),
			stage: IdentityStage::Registered,
			expiration_time: 10000,
			proof: None,
			metadata: None
		}
	}

	fn build_identity_hash(identity_type: &[u8], identity: &[u8]) -> H256 {
			let mut buf = Vec::new();
			buf.extend_from_slice(&identity_type.to_vec().encode());
			buf.extend_from_slice(&identity.to_vec().encode());
			return Blake2Hasher::hash(&buf[..]);
	}

	#[test]
	fn register_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

 			let expiration_time = Identity::expiration_time();
			let now = Timestamp::get();
			let expires_at = now + expiration_time;

			assert_ok!(register_identity(public, identity_type, identity));
			assert_eq!(
				System::events(),
				vec![EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: Event::identity(RawEvent::Register(identity_hash, public, expires_at))
				}]
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(default_identity_record(public, identity_type, identity))
			);
		});
	}

	#[test]
	fn register_twice_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));
			assert_err!(
				register_identity(public, identity_type, identity),
				"Identity type already used"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(default_identity_record(public, identity_type, identity))
			);
		});
	}

	#[test]
	fn register_existing_identity_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));

			let pair2: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();
			let public2: H256 = pair2.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));
			assert_err!(
				register_identity(public2, identity_type, identity),
				"Identity already exists"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(default_identity_record(public, identity_type, identity))
			);
		});
	}

	#[test]
	fn register_same_type_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

			let new_github: &[u8] = b"drstone";
			assert_err!(
				register_identity(public, identity_type, new_github),
				"Identity type already used"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(default_identity_record(public, identity_type, identity))
			);
		});
	}

	#[test]
	fn register_and_attest_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

 			let mut expiration_time = Identity::expiration_time();
			let mut now = Timestamp::get();
			let register_expires_at = now + expiration_time;

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_ok!(attest_to_identity(public, identity_hash, attestation));

 			expiration_time = Identity::expiration_time();
			now = Timestamp::get();
			let attest_expires_at = now + expiration_time;

			assert_eq!(
				System::events(),
				vec![
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Register(identity_hash, public, register_expires_at))
					},
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Attest(identity_hash, public, attest_expires_at))
					}
				]
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(IdentityRecord {
					stage: IdentityStage::Attested,
					proof: Some(attestation.to_vec()),
					..default_identity_record(public, identity_type, identity)
				})
			);
		});
	}

	#[test]
	fn attest_without_register_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_err!(
				attest_to_identity(public, identity_hash, attestation),
				"Identity does not exist"
			);
			assert_eq!(Identity::identities(), vec![]);
			assert_eq!(Identity::identities_pending(), vec![]);
			assert_eq!(Identity::identity_of(identity_hash), None);
		});
	}

	#[test]
	fn attest_from_different_account_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let other: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();
			let other_pub: H256 = other.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));
			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_err!(
				attest_to_identity(other_pub, identity_hash, attestation),
				"Stored identity does not match sender"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(default_identity_record(public, identity_type, identity))
			);
		});
	}

	#[test]
	fn register_attest_and_verify_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

 			let mut expiration_time = Identity::expiration_time();
			let mut now = Timestamp::get();
			let register_expires_at = now + expiration_time;

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_ok!(attest_to_identity(public, identity_hash, attestation));

 			expiration_time = Identity::expiration_time();
			now = Timestamp::get();
			let attest_expires_at = now + expiration_time;

			let verifier = H256::from_low_u64_be(1);
			assert_ok!(verify_identity(verifier, identity_hash, true, 0));

			assert_eq!(
				System::events(),
				vec![
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Register(identity_hash, public, register_expires_at))
					},
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Attest(identity_hash, public, attest_expires_at))
					},
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Verify(identity_hash, verifier))
					}
				]
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(IdentityRecord {
					stage: IdentityStage::Verified,
					expiration_time: 0,
					proof: Some(attestation.to_vec()),
					..default_identity_record(public, identity_type, identity)
				})
			);
		});
	}

	#[test]
	fn attest_after_verify_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_ok!(attest_to_identity(public, identity_hash, attestation));

			let verifier = H256::from_low_u64_be(1);
			assert_ok!(verify_identity(verifier, identity_hash, true, 0));
			assert_err!(
				attest_to_identity(public, identity_hash, attestation),
				"Already verified"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(IdentityRecord {
					stage: IdentityStage::Verified,
					expiration_time: 0,
					proof: Some(attestation.to_vec()),
					..default_identity_record(public, identity_type, identity)
				})
			);
		});
	}

	#[test]
	fn verify_from_nonverifier_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_ok!(attest_to_identity(public, identity_hash, attestation));

			assert_err!(
				verify_identity(public, identity_hash, true, 0),
				"Sender is not a verifier"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(IdentityRecord {
					stage: IdentityStage::Attested,
					proof: Some(attestation.to_vec()),
					..default_identity_record(public, identity_type, identity)
				})
			);
		});
	}

	#[test]
	fn verify_from_wrong_index_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_ok!(attest_to_identity(public, identity_hash, attestation));

			assert_err!(
				verify_identity(public, identity_hash, true, 1),
				"Verifier index out of bounds"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(IdentityRecord {
					stage: IdentityStage::Attested,
					proof: Some(attestation.to_vec()),
					..default_identity_record(public, identity_type, identity)
				})
			);
		});
	}

	#[test]
	fn register_should_expire() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			assert_ok!(register_identity(public, identity_type, identity));

 			let expiration_time = Identity::expiration_time();
			let now = Timestamp::get();
			let expires_at = now + expiration_time;

			Timestamp::set_timestamp(10001);

			<Identity as OnFinalise<u64>>::on_finalise(1);
			System::set_block_number(2);

			let attestation: &[u8] = b"www.proof.com/attest_of_extra_proof";
			assert_err!(
				attest_to_identity(public, identity_hash, attestation),
				"Identity does not exist"
			);

			assert_eq!(
				System::events(),
				vec![
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Register(identity_hash, public, expires_at))
					},
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::identity(RawEvent::Expired(identity_hash))
					},
				]
			);
			assert_eq!(Identity::identities(), vec![]);
			assert_eq!(Identity::identities_pending(), vec![]);
			assert_eq!(Identity::identity_of(identity_hash), None);
		});
	}

	#[test]
	fn add_metadata_should_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);

			let public: H256 = pair.public().0.into();

			let avatar: &[u8] = b"avatars3.githubusercontent.com/u/13153687";
			let display_name: &[u8] = b"drewstone";
			let tagline: &[u8] = b"hello world!";

			assert_ok!(register_identity(public, identity_type, identity));
			assert_ok!(add_metadata_to_account(
				public,
				identity_hash,
				avatar,
				display_name,
				tagline
			));
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			let default_record = default_identity_record(public, identity_type, identity);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(IdentityRecord {
					metadata: Some(MetadataRecord {
					avatar: avatar.to_vec(),
					display_name: display_name.to_vec(),
					tagline: tagline.to_vec(),
					}),
					..default_record
				})
			);
		});
	}

	#[test]
	fn add_metadata_without_register_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();

			let avatar: &[u8] = b"avatars3.githubusercontent.com/u/13153687";
			let display_name: &[u8] = b"drewstone";
			let tagline: &[u8] = b"hello world!";
			assert_err!(
				add_metadata_to_account(public, identity_hash, avatar, display_name, tagline),
				"Identity does not exist"
			);
			assert_eq!(Identity::identities(), vec![]);
			assert_eq!(Identity::identities_pending(), vec![]);
			assert_eq!(Identity::identity_of(identity_hash), None);
		});
	}

	#[test]
	fn add_metadata_from_different_account_should_not_work() {
		with_externalities(&mut new_test_ext(), || {
			System::set_block_number(1);

			let pair: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
			));
			let other: Pair = Pair::from_seed(&hex!(
				"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f61"
			));
			let identity_type: &[u8] = b"github";
			let identity: &[u8] = b"drewstone";
			let identity_hash = build_identity_hash(identity_type, identity);
			let public: H256 = pair.public().0.into();
			let other_pub: H256 = other.public().0.into();

			let avatar: &[u8] = b"avatars3.githubusercontent.com/u/13153687";
			let display_name: &[u8] = b"drewstone";
			let tagline: &[u8] = b"hello world!";

			assert_ok!(register_identity(public, identity_type, identity));
			assert_err!(
				add_metadata_to_account(other_pub, identity_hash, avatar, display_name, tagline),
				"Stored identity does not match sender"
			);
			assert_eq!(Identity::identities(), vec![identity_hash]);
			assert_eq!(Identity::identities_pending(), vec![(identity_hash, 10000)]);
			assert_eq!(
				Identity::identity_of(identity_hash),
				Some(default_identity_record(public, identity_type, identity))
			);
		});
	}
}