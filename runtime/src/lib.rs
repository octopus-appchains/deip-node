#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use beefy_primitives::{crypto::AuthorityId as BeefyId, mmr::MmrLeafVersion, ValidatorSet};
pub use frame_support::{
    construct_runtime,
    pallet_prelude::{
        DispatchClass, Encode, InherentData, TransactionPriority, TransactionSource,
        TransactionValidity, Weight,
    },
    parameter_types,
    traits::{Everything, KeyOwnerProofSystem, StorageInfo},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee,
    },
    PalletId,
};
use frame_system::{
    self,
    limits::{BlockLength, BlockWeights},
    EnsureRoot,
};
pub use pallet_balances::Call as BalancesCall;
use pallet_deip::{InvestmentId, ProjectId, H160};
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::CurrencyAdapter;
use sp_api::{impl_runtime_apis, BlockT, NumberFor, RuntimeVersion};
use sp_core::OpaqueMetadata;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
    create_runtime_str,
    generic::{self, Era},
    impl_opaque_keys,
    traits::{
        AccountIdLookup, BlakeTwo256, ConvertInto, Extrinsic, IdentifyAccount, Keccak256,
        OpaqueKeys, SaturatedConversion, StaticLookup, Verify,
    },
    ApplyExtrinsicResult, KeyTypeId, MultiSignature,
};
pub use sp_runtime::{Perbill, Permill};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;

pub mod deip_account;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

pub type DeipAssetId = H160;
pub type AssetId = u32;
/// Balance of an UIAs.
pub type AssetBalance = u128;
pub type AssetExtra = ();

/// Identifier for the class of the NFT asset.
pub type NftClassId = u32; // ??? what is class id right type

/// Deip indentifier for the class of the NFT asset.
pub type DeipNftClassId = H160;

/// The type used to identify a unique asset within an asset class.
pub type InstanceId = u32; // ??? correct type

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
// pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub babe: Babe,
            pub grandpa: Grandpa,
            pub im_online: ImOnline,
            pub beefy: Beefy,
            pub octopus: OctopusAppchain,
        }
    }
}

// To learn more about runtime versioning and what each of the following value means:
//   https://substrate.dev/docs/en/knowledgebase/runtime/upgrades#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("appchain"),
    impl_name: create_runtime_str!("appchain-deip"),
    authoring_version: 1,
    // The version of the runtime specification. A full node will not attempt to use its native
    //   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
    //   `spec_version`, and `authoring_version` are the same between Wasm and native.
    // This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
    //   the compatible custom types.
    spec_version: 102,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
};

/// Since BABE is probabilistic this is the average expected block time that
/// we are targeting. Blocks will be produced at a minimum duration defined
/// by `SLOT_DURATION`, but some slots will not be allocated to any
/// authority and hence no block will be produced. We expect to have this
/// block time on average following the defined slot duration and the value
/// of `c` configured for BABE (where `1 - c` represents the probability of
/// a slot being empty).
/// This value is only used indirectly to define the unit constants below
/// that are expressed in blocks. The rest of the code should use
/// `SLOT_DURATION` instead (like the Timestamp pallet for calculating the
/// minimum period).
///
/// If using BABE with secondary slots (default) then all of the slots will
/// always be assigned, in which case `MILLISECS_PER_BLOCK` and
/// `SLOT_DURATION` should have the same value.
///
/// <https://research.web3.foundation/en/latest/polkadot/block-production/Babe.html#-6.-practical-results>
pub const MILLISECS_PER_BLOCK: Moment = 6000;
pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
//       Attempting to do so will brick block production.
pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 4 * HOURS;
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
    const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

    (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
};

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub mod currency {
    use super::Balance;

    pub const OCTS: Balance = 1_000_000_000_000_000_000;
    pub const UNITS: Balance = 1_000_000_000_000_000_000;

    pub const DOLLARS: Balance = UNITS;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    pub const EXISTENSIAL_DEPOSIT: Balance = CENTS;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
    }
}

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
    };

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;

    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = Everything;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
}

impl pallet_randomness_collective_flip::Config for Runtime {}

impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type KeyOwnerProofSystem = Historical;
    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;
    type HandleEquivocation =
        pallet_grandpa::EquivocationHandler<Self::KeyOwnerIdentification, (), ReportLongevity>;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = currency::EXISTENSIAL_DEPOSIT;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * currency::MILLICENTS;
    pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_deip_ecosystem_fund::Config for Runtime {}

use pallet_deip_ecosystem_fund::DontBurnFee;

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = CurrencyAdapter<Balances, DontBurnFee<(Self, Balances)>>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
    // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
    pub const ReportLongevity: u64 =
        BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
    pub const MaxAuthorities: u32 = 100;
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;

    type KeyOwnerProofSystem = Historical;

    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::IdentificationTuple;

    type HandleEquivocation =
        pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, (), ReportLongevity>;

    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 0;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (OctopusLpos, ImOnline);
}

parameter_types! {
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Config for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = pallet_session_historical::NoteHistoricalRoot<Self, OctopusLpos>;
    type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = opaque::SessionKeys;
    // type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session_historical::Config for Runtime {
    type FullIdentification = u128;
    type FullIdentificationOf = pallet_octopus_lpos::ExposureOf<Runtime>;
}

parameter_types! {
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
//     /// We prioritize im-online heartbeats over election solution submission.
//     pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
    pub const MaxKeys: u32 = 10_000;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
    pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        public: <Signature as Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
        let tip = 0;
        // take the biggest period possible.
        let period =
            BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let era = Era::mortal(period, current_block);
        let extra = (
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(era),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                log::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = <Self as frame_system::Config>::Lookup::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type Event = Event;
    type NextSessionRotation = Babe;
    type ValidatorSet = Historical;
    type ReportUnresponsiveness = ();
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
    type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
}

type MmrHash = <Keccak256 as sp_runtime::traits::Hash>::Output;

/// A BEEFY consensus digest item with MMR root hash.
pub struct DepositLog;
impl pallet_mmr::primitives::OnNewRoot<MmrHash> for DepositLog {
    fn on_new_root(root: &Hash) {
        let digest = generic::DigestItem::Consensus(
            beefy_primitives::BEEFY_ENGINE_ID,
            codec::Encode::encode(&beefy_primitives::ConsensusLog::<BeefyId>::MmrRoot(*root)),
        );
        <frame_system::Pallet<Runtime>>::deposit_log(digest);
    }
}

impl pallet_mmr::Config for Runtime {
    const INDEXING_PREFIX: &'static [u8] = b"mmr";
    type Hashing = Keccak256;
    type Hash = MmrHash;
    type LeafData = pallet_beefy_mmr::Pallet<Self>;
    type OnNewRoot = pallet_beefy_mmr::DepositBeefyDigest<Self>;
    type WeightInfo = ();
}

parameter_types! {
    pub const AssetDeposit: Balance = 100 * currency::UNITS;
    pub const ApprovalDeposit: Balance = 1 * currency::UNITS;
    pub const StringLimit: u32 = 200;
    pub const MetadataDepositBase: Balance = 10 * currency::UNITS;
    pub const MetadataDepositPerByte: Balance = 1 * currency::UNITS;
}

impl pallet_assets::Config for Runtime {
    type Event = Event;
    type Balance = AssetBalance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = AssetExtra;
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

impl deip_projects_info::DeipProjectsInfo<AccountId> for Runtime {
    type ProjectId = pallet_deip::ProjectId;
    type InvestmentId = pallet_deip::InvestmentId;

    fn try_get_project_team(id: &Self::ProjectId) -> Option<AccountId> {
        Deip::try_get_project_team(id)
    }

    fn project_id(source: &[u8]) -> Self::ProjectId {
        Self::ProjectId::from_slice(source)
    }
}

parameter_types! {
    pub const WipePeriod: BlockNumber = DAYS;
}

pub struct AssetIdInit;
impl deip_asset_system::AssetIdInitT<DeipAssetId> for AssetIdInit {
    fn asset_id(raw: &[u8]) -> DeipAssetId {
        DeipAssetId::from_slice(raw)
    }
}

impl pallet_deip_assets::Config for Runtime {
    type ProjectsInfo = Self;
    type DeipAccountId = deip_account::DeipAccountId<Self::AccountId>;
    type AssetsAssetId = AssetId;
    type AssetId = DeipAssetId;
    type AssetIdInit = AssetIdInit;
    type WipePeriod = WipePeriod;
}

impl pallet_deip_balances::Config for Runtime {}

parameter_types! {
    /// The basic amount of funds that must be reserved for an asset class.
    pub const ClassDeposit: Balance = 10 * currency::UNITS;

    /// The basic amount of funds that must be reserved for an asset instance.
    pub const InstanceDeposit: Balance = 10 * currency::UNITS;

    /// The basic amount of funds that must be reserved when adding an attribute to an asset.
    pub const AttributeDepositBase: Balance = 10 * currency::UNITS;

    /// The additional funds that must be reserved for the number of bytes store in metadata,
    /// either "normal" metadata or attribute metadata.
    pub const DepositPerByte: Balance = 10 * currency::UNITS;

    /// The maximum length of an attribute key.
    pub const KeyLimit: u32 = 100;

    /// The maximum length of an attribute value.
    pub const ValueLimit: u32 = 200;

    /// Greater class ids will be reserved for `deip_*` calls.
    pub const MaxOriginClassId: NftClassId = NftClassId::MAX / 2;
}

impl pallet_uniques::Config for Runtime {
    type Event = Event;
    type ClassId = NftClassId;
    type InstanceId = InstanceId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type ClassDeposit = ClassDeposit;
    type InstanceDeposit = InstanceDeposit;
    type MetadataDepositBase = MetadataDepositBase; // ??? is it correct to reuse const from assets
    type AttributeDepositBase = AttributeDepositBase;
    type DepositPerByte = DepositPerByte;
    type StringLimit = StringLimit; // ??? is it correct to reuse const from assets
    type KeyLimit = KeyLimit;
    type ValueLimit = ValueLimit;
    type WeightInfo = pallet_uniques::weights::SubstrateWeight<Runtime>;
}

impl pallet_deip_uniques::Config for Runtime {
    type DeipNftClassId = DeipNftClassId;
    type DeipAccountId = deip_account::DeipAccountId<<Self as frame_system::Config>::AccountId>;
    type ProjectId = pallet_deip::ProjectId;
    type NftClassId = <Self as pallet_uniques::Config>::ClassId;
    type ProjectsInfo = Self;
    type MaxOriginClassId = MaxOriginClassId;
}

impl pallet_beefy::Config for Runtime {
    type BeefyId = BeefyId;
}

parameter_types! {
    pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0,0);
}

impl pallet_beefy_mmr::Config for Runtime {
    type LeafVersion = LeafVersion;
    type BeefyAuthorityToMerkleLeaf = pallet_beefy_mmr::BeefyEcdsaToEthereum;
    type ParachainHeads = ();
}

pub struct OctopusAppCrypto;

impl frame_system::offchain::AppCrypto<<Signature as Verify>::Signer, Signature>
    for OctopusAppCrypto
{
    type RuntimeAppPublic = pallet_octopus_appchain::AuthorityId;
    type GenericSignature = sp_core::sr25519::Signature;
    type GenericPublic = sp_core::sr25519::Public;
}

parameter_types! {
    pub const OctopusAppchainPalletId: PalletId = PalletId(*b"py/octps");
    pub const GracePeriod: u32 = 10;
    pub const UnsignedPriority: u64 = 1 << 21;
    pub const RequestEventLimit: u32 = 10;
    pub const UpwardMessagesLimit: u32 = 10;
}

impl pallet_octopus_appchain::Config for Runtime {
    type AuthorityId = OctopusAppCrypto;
    type Event = Event;
    type Call = Call;
    type PalletId = OctopusAppchainPalletId;
    type LposInterface = OctopusLpos;
    type UpwardMessagesInterface = OctopusUpwardMessages;
    type Currency = Balances;
    type Assets = Assets; // @TODO replace with deip assets
    type AssetBalance = AssetBalance;
    type AssetId = AssetId;
    type AssetIdByName = OctopusAppchain;
    type GracePeriod = GracePeriod;
    type UnsignedPriority = UnsignedPriority;
    type RequestEventLimit = RequestEventLimit;
    type WeightInfo = pallet_octopus_appchain::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
        pub const SessionsPerEra: sp_staking::SessionIndex = 6;
        pub const BondingDuration: pallet_octopus_lpos::EraIndex = 24 * 28;
//         pub OffchainRepeat: BlockNumber = 5;
        pub const BlocksPerEra: u32 = EPOCH_DURATION_IN_BLOCKS * 6;
}

impl pallet_octopus_lpos::Config for Runtime {
    type Currency = Balances;
    type UnixTime = Timestamp;
    type Event = Event;
    type Reward = (); // rewards are minted from the void
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type BlocksPerEra = BlocksPerEra;
    type SessionInterface = Self;
    type AppchainInterface = OctopusAppchain;
    type UpwardMessagesInterface = OctopusUpwardMessages;
    type PalletId = OctopusAppchainPalletId;
    type ValidatorsProvider = OctopusAppchain;
    type WeightInfo = pallet_octopus_lpos::weights::SubstrateWeight<Runtime>;
}

impl pallet_octopus_upward_messages::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type UpwardMessagesLimit = UpwardMessagesLimit;
    type WeightInfo = pallet_octopus_upward_messages::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

pub type TransactionCtx = pallet_deip_portal::PortalCtxOf<Runtime>;
pub type TransactionCtxId = pallet_deip_portal::TransactionCtxId<TransactionCtx>;

parameter_types! {
    pub const MaxNdaParties: u16 = 50;
    pub const MaxInvestmentShares: u16 = 10;
}

impl pallet_deip::Config for Runtime {
    type TransactionCtx = TransactionCtx;
    type Event = Event;
    type DeipAccountId = deip_account::DeipAccountId<Self::AccountId>;
    type Currency = Balances;
    type AssetSystem = Self;
    type DeipWeightInfo = pallet_deip::Weights<Self>;
    type MaxNdaParties = MaxNdaParties;
    type MaxInvestmentShares = MaxInvestmentShares;
}

parameter_types! {
    pub const ProposalTtl: Moment = 7 * DAYS as Moment * MILLISECS_PER_BLOCK;
    pub const ProposalExpirePeriod: BlockNumber = HOURS;
}

impl pallet_deip_proposal::pallet::Config for Runtime {
    type TransactionCtx = TransactionCtx;
    type Event = Event;
    type Call = Call;
    type DeipAccountId = deip_account::DeipAccountId<Self::AccountId>;
    type Ttl = ProposalTtl;
    type ExpirePeriod = ProposalExpirePeriod;
    type WeightInfo = pallet_deip_proposal::CallWeight<Self>;
}

parameter_types! {
    pub const DaoMaxSignatories: u16 = 50;
}

impl pallet_deip_dao::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type DaoId = pallet_deip_dao::DaoId;
    type DeipDaoWeightInfo = pallet_deip_dao::weights::Weights<Self>;
    type MaxSignatories = DaoMaxSignatories;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = currency::deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = currency::deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
    type PalletsOrigin = OriginCaller;
}

impl pallet_deip_portal::Config for Runtime {
    type TenantLookup = Self;
    type PortalId = <Runtime as pallet_deip_dao::Config>::DaoId;
    type Portal = pallet_deip_portal::Portal<Self>;

    type Call = Call;
    type UnsignedValidator = Self;
    type UncheckedExtrinsic = UncheckedExtrinsic;
    type DeipPortalWeightInfo = pallet_deip_portal::weights::Weights<Self>;
}

impl pallet_deip_portal::TenantLookupT<AccountId> for Runtime {
    type TenantId = <Self as pallet_deip_portal::Config>::PortalId;

    fn lookup(key: &AccountId) -> Option<Self::TenantId> {
        DeipDao::lookup_dao(key)
    }
}

impl deip_asset_system::AssetIdInitT<DeipAssetId> for Runtime {
    fn asset_id(raw: &[u8]) -> DeipAssetId {
        DeipAssetId::from_slice(raw)
    }
}

impl pallet_deip::traits::DeipAssetSystem<AccountId> for Runtime {
    type Balance = AssetBalance;
    type AssetId = DeipAssetId;

    fn try_get_tokenized_project(id: &Self::AssetId) -> Option<ProjectId> {
        Assets::try_get_tokenized_project(id)
    }

    fn account_balance(account: &AccountId, asset: &Self::AssetId) -> Self::Balance {
        Assets::account_balance(account, asset)
    }

    fn total_supply(asset: &Self::AssetId) -> Self::Balance {
        Assets::total_supply(asset)
    }

    fn get_project_fts(id: &ProjectId) -> Vec<Self::AssetId> {
        Assets::get_project_fts(id)
    }

    fn get_ft_balances(id: &Self::AssetId) -> Option<Vec<AccountId>> {
        Assets::get_ft_balances(id)
    }

    fn transactionally_transfer(
        from: &AccountId,
        asset: Self::AssetId,
        transfers: &[(Self::Balance, AccountId)],
    ) -> Result<(), ()> {
        Assets::transactionally_transfer(from, asset, transfers)
    }

    fn transactionally_reserve(
        account: &AccountId,
        id: InvestmentId,
        shares: &[(Self::AssetId, Self::Balance)],
        asset: Self::AssetId,
    ) -> Result<(), deip_assets_error::ReserveError<Self::AssetId>> {
        Assets::deip_transactionally_reserve(account, id, shares, asset)
    }

    fn transactionally_unreserve(
        id: InvestmentId,
    ) -> Result<(), deip_assets_error::UnreserveError<Self::AssetId>> {
        Assets::transactionally_unreserve(id)
    }

    fn transfer_from_reserved(
        id: InvestmentId,
        who: &AccountId,
        asset: Self::AssetId,
        amount: Self::Balance,
    ) -> Result<(), deip_assets_error::UnreserveError<Self::AssetId>> {
        Assets::transfer_from_reserved(id, who, asset, amount)
    }

    fn transfer_to_reserved(
        who: &AccountId,
        id: InvestmentId,
        amount: Self::Balance,
    ) -> Result<(), deip_assets_error::UnreserveError<Self::AssetId>> {
        Assets::deip_transfer_to_reserved(who, id, amount)
    }
}

parameter_types! {
  pub const MinVestedTransfer: u64 = 1;
}

impl pallet_deip_vesting::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type UnixTime = Timestamp;
    type MinVestedTransfer = MinVestedTransfer;
    type U64ToBalance = ConvertInto;
    type VestingWeightInfo = pallet_deip_vesting::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub enum Runtime
        where
            Block = Block,
            NodeBlock = opaque::Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Babe: pallet_babe,
        Timestamp: pallet_timestamp,
        Authorship: pallet_authorship,
        Balances: pallet_deip_balances,
        TransactionPayment: pallet_transaction_payment,
        OctopusAppchain: pallet_octopus_appchain::{Pallet, Call, Storage, Config<T>, Event<T>, ValidateUnsigned},
        OctopusLpos: pallet_octopus_lpos,
        OctopusUpwardMessages: pallet_octopus_upward_messages::{Pallet, Call, Storage, Event<T>},
        Session: pallet_session,
        Grandpa: pallet_grandpa,
        Sudo: pallet_sudo,
        ImOnline: pallet_im_online,
        Historical: pallet_session_historical::{Pallet},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
        ParityTechAssets: pallet_assets::{Pallet, Storage, Event<T>},
        ParityTechBalances: pallet_balances::{Pallet, Storage, Event<T>, Config<T>},
        ParityTechUniques: pallet_uniques::{Pallet, Storage, Event<T>},
        Mmr: pallet_mmr::{Pallet, Storage},
        Beefy: pallet_beefy::{Pallet, Config<T>},
        MmrLeaf: pallet_beefy_mmr,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>},
        Utility: pallet_utility::{Pallet, Call, Event},
        Deip: pallet_deip::{Pallet, Call, Storage, Event<T>, Config, ValidateUnsigned},
        Assets: pallet_deip_assets::{Pallet, Storage, Call, Config<T>, ValidateUnsigned},
        Uniques: pallet_deip_uniques::{Pallet, Storage, Call, Config<T>},
        DeipProposal: pallet_deip_proposal::{Pallet, Call, Storage, Event<T>, Config, ValidateUnsigned},
        DeipDao: pallet_deip_dao::{Pallet, Call, Storage, Event<T>, Config},
        DeipPortal: pallet_deip_portal::{Pallet, Call, Storage, Config, ValidateUnsigned},
        DeipVesting: pallet_deip_vesting::{Pallet, Call, Storage, Event<T>, Config<T>},
        DeipEcosystemFund: pallet_deip_ecosystem_fund::{Pallet, Config<T>, Storage},
    }
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
>;

/// MMR helper types.
mod mmr {
    use super::Runtime;
    pub use pallet_mmr::primitives::*;

    pub type Leaf = <<Runtime as pallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
    pub type Hash = <Runtime as pallet_mmr::Config>::Hash;
    pub type Hashing = <Runtime as pallet_mmr::Config>::Hashing;
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Grandpa::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((fg_primitives::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(fg_primitives::OpaqueKeyOwnershipProof::new)
        }

        fn current_set_id() -> u64 {
            todo!()
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            sp_consensus_babe::BabeGenesisConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: BABE_GENESIS_EPOCH_CONFIG.c,
                genesis_authorities: Babe::authorities().to_vec(),
                randomness: Babe::randomness(),
                allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Babe::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    impl pallet_mmr::primitives::MmrApi<
        Block,
        mmr::Hash,
    > for Runtime {
        fn generate_proof(leaf_index: u64)
            -> Result<(mmr::EncodableOpaqueLeaf, mmr::Proof<mmr::Hash>), mmr::Error>
        {
            Mmr::generate_proof(leaf_index)
                .map(|(leaf, proof)| (mmr::EncodableOpaqueLeaf::from_leaf(&leaf), proof))
        }

        fn verify_proof(leaf: mmr::EncodableOpaqueLeaf, proof: mmr::Proof<mmr::Hash>)
            -> Result<(), mmr::Error>
        {
            let leaf: mmr::Leaf = leaf
                .into_opaque_leaf()
                .try_decode()
                .ok_or(mmr::Error::Verify)?;
            Mmr::verify_leaf(leaf, proof)
        }

        fn verify_proof_stateless(
            root: mmr::Hash,
            leaf: mmr::EncodableOpaqueLeaf,
            proof: mmr::Proof<mmr::Hash>
        ) -> Result<(), mmr::Error> {
            let node = mmr::DataOrHash::Data(leaf.into_opaque_leaf());
            pallet_mmr::verify_leaf_proof::<mmr::Hashing, _>(root, node, proof)
        }
    }

    impl beefy_primitives::BeefyApi<Block> for Runtime {
        fn validator_set() -> ValidatorSet<BeefyId> {
            Beefy::validator_set()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{list_benchmark, baseline, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            let mut list = Vec::<BenchmarkList>::new();

            list_benchmark!(list, extra, frame_benchmarking, BaselineBench::<Runtime>);
            list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
            list_benchmark!(list, extra, pallet_timestamp, Timestamp);
            list_benchmark!(list, extra, pallet_deip_proposal, DeipProposal);
            list_benchmark!(list, extra, pallet_deip_dao, DeipDao);
            list_benchmark!(list, extra, pallet_deip_portal, DeipPortal);
            list_benchmark!(list, extra, pallet_deip, Deip);

            let storage_info = AllPalletsWithSystem::storage_info();

            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, frame_benchmarking, BaselineBench::<Runtime>);
            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);
            add_benchmark!(params, batches, pallet_deip_proposal, DeipProposal);
            add_benchmark!(params, batches, pallet_deip_dao, DeipDao);
            add_benchmark!(params, batches, pallet_deip_portal, DeipPortal);
            add_benchmark!(params, batches, pallet_deip, Deip);

            Ok(batches)
        }
    }

    impl pallet_deip_dao::api::DeipDaoRuntimeApi<Block, AccountId> for Runtime {
        fn get(name: pallet_deip_dao::dao::DaoId) -> pallet_deip_dao::api::GetResult<AccountId> {
            DeipDao::rpc_get(name)
        }

        fn get_multi(names: Vec<pallet_deip_dao::dao::DaoId>) -> pallet_deip_dao::api::GetMultiResult<AccountId> {
            DeipDao::rpc_get_multi(names)
        }
    }

    impl pallet_deip::api::DeipApi
    <
        Block,
        AccountId,
        Moment,
        DeipAssetId,
        AssetBalance,
        Hash,
        pallet_deip_portal::TransactionCtxId<TransactionCtx>
    >
    for Runtime {
        fn get_project(project_id: &ProjectId) -> Option<pallet_deip::ProjectOf<crate::Runtime>> {
            Deip::get_project(project_id)
        }

        fn get_project_content(id: &pallet_deip::ProjectContentId) -> Option<pallet_deip::ProjectContentOf<crate::Runtime>> {
            Deip::get_project_content(id)
        }

        fn get_domain(domain_id: &pallet_deip::DomainId) -> Option<pallet_deip::Domain> {
            Deip::get_domain(domain_id)
        }

        fn get_nda(nda_id: &pallet_deip::NdaId) -> Option<pallet_deip::NdaOf<crate::Runtime>> {
            Deip::get_nda(nda_id)
        }

        fn get_review(id: &pallet_deip::ReviewId) -> Option<pallet_deip::ReviewOf<crate::Runtime>> {
            Deip::get_review(id)
        }

        fn get_contract_agreement(id: &pallet_deip::ContractAgreementId) -> Option<pallet_deip::ContractAgreementOf<crate::Runtime>> {
            Deip::get_contract_agreement(id)
        }
    }
}
