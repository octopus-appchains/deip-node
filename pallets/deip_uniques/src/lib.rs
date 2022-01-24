// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use pallet_uniques;

#[frame_support::pallet]
pub mod pallet {
    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;

    use frame_support::{
        dispatch::{DispatchResult, DispatchResultWithPostInfo, UnfilteredDispatchable, Vec},
        ensure,
        pallet_prelude::{OptionQuery, StorageMap, StorageValue, ValueQuery},
        sp_runtime::traits::{CheckedAdd, One, StaticLookup},
        BoundedVec, Identity, Parameter,
    };
    use frame_system::pallet_prelude::OriginFor;
    use pallet_uniques::{DestroyWitness, WeightInfo};

    // Helper types.
    type DeipNftClassIdOf<T> = <T as Config>::NftClassId;
    type DeipProjectIdOf<T> = <T as Config>::ProjectId;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_uniques::Config<ClassId = Self::UniquesNftClassId>
    {
        /// Deip class id.
        type NftClassId: Parameter + Copy;

        /// Deip account id.
        type AccountId: Parameter;

        /// Deip project id.
        type ProjectId: Parameter;

        /// Type of `pallet_uniques::Config::ClassId`.
        type UniquesNftClassId: Parameter + CheckedAdd + Default + One + Copy;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::genesis_config]
    pub struct GenesisConfig<T> {
        pub _marker: std::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            NextNftClassId::<T>::put(<T as pallet_uniques::Config>::ClassId::default());
        }
    }

    #[cfg(feature = "std")]
    impl<T> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { _marker: std::marker::PhantomData }
        }
    }

    /// Storage for matching Deip class id and origin `pallet_uniques` class id.
    #[pallet::storage]
    pub type NftClassIdByDeipNftClassId<T: Config> = StorageMap<
        _,
        Identity,
        DeipNftClassIdOf<T>,
        <T as pallet_uniques::Config>::ClassId,
        OptionQuery,
    >;

    /// Storage for next NFT origin class id.
    #[pallet::storage]
    pub(super) type NextNftClassId<T> =
        StorageValue<_, <T as Config>::UniquesNftClassId, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        DeipNftClassIdExists,
        NftClassIdOverflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Issue a new class of non-fungible assets from a public origin.
        #[pallet::weight(<T as pallet_uniques::Config>::WeightInfo::create())]
        pub fn create(
            origin: OriginFor<T>,
            class: <T as pallet_uniques::Config>::ClassId,
            admin: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::create(origin, class, admin)
        }

        #[pallet::weight(<T as pallet_uniques::Config>::WeightInfo::force_create())]
        pub fn force_create(
            origin: OriginFor<T>,
            class: T::ClassId,
            owner: <T::Lookup as StaticLookup>::Source,
            free_holding: bool,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::force_create(origin, class, owner, free_holding)
        }

        /// Destroy a class of fungible assets.
        #[pallet::weight(pallet_uniques::Call::<T>::destroy{class: *class, witness: *witness}.get_dispatch_info().weight)]
        pub fn destroy(
            origin: OriginFor<T>,
            class: T::ClassId,
            witness: DestroyWitness,
        ) -> DispatchResultWithPostInfo {
            pallet_uniques::Pallet::<T>::destroy(origin, class, witness)
        }

        /// Mint an asset instance of a particular class.
        #[pallet::weight(T::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
            owner: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::mint(origin, class, instance, owner)
        }

        /// Destroy a single asset instance.
        #[pallet::weight(T::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
            check_owner: Option<<T::Lookup as StaticLookup>::Source>,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::burn(origin, class, instance, check_owner)
        }

        /// Move an asset from the sender account to another.
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
            dest: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::transfer(origin, class, instance, dest)
        }

        /// Reevaluate the deposits on some assets.
        #[pallet::weight(T::WeightInfo::redeposit(instances.len() as u32))]
        pub fn redeposit(
            origin: OriginFor<T>,
            class: T::ClassId,
            instances: Vec<T::InstanceId>,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::redeposit(origin, class, instances)
        }

        /// Disallow further unprivileged transfer of an asset instance.
        #[pallet::weight(T::WeightInfo::freeze())]
        pub fn freeze(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::freeze(origin, class, instance)
        }

        /// Re-allow unprivileged transfer of an asset instance.
        #[pallet::weight(T::WeightInfo::thaw())]
        pub fn thaw(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::thaw(origin, class, instance)
        }

        /// Disallow further unprivileged transfers for a whole asset class.
        #[pallet::weight(T::WeightInfo::freeze_class())]
        pub fn freeze_class(origin: OriginFor<T>, class: T::ClassId) -> DispatchResult {
            pallet_uniques::Pallet::<T>::freeze_class(origin, class)
        }

        /// Re-allow unprivileged transfers for a whole asset class.
        #[pallet::weight(T::WeightInfo::thaw_class())]
        pub fn thaw_class(origin: OriginFor<T>, class: T::ClassId) -> DispatchResult {
            pallet_uniques::Pallet::<T>::thaw_class(origin, class)
        }

        /// Change the Owner of an asset class.
        #[pallet::weight(T::WeightInfo::transfer_ownership())]
        pub fn transfer_ownership(
            origin: OriginFor<T>,
            class: T::ClassId,
            owner: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::transfer_ownership(origin, class, owner)
        }

        /// Change the Issuer, Admin and Freezer of an asset class.
        #[pallet::weight(T::WeightInfo::set_team())]
        pub fn set_team(
            origin: OriginFor<T>,
            class: T::ClassId,
            issuer: <T::Lookup as StaticLookup>::Source,
            admin: <T::Lookup as StaticLookup>::Source,
            freezer: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::set_team(origin, class, issuer, admin, freezer)
        }

        /// Approve an instance to be transferred by a delegated third-party account.
        #[pallet::weight(T::WeightInfo::approve_transfer())]
        pub fn approve_transfer(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
            delegate: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::approve_transfer(origin, class, instance, delegate)
        }

        /// Cancel the prior approval for the transfer of an asset by a delegate.
        #[pallet::weight(T::WeightInfo::cancel_approval())]
        pub fn cancel_approval(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
            maybe_check_delegate: Option<<T::Lookup as StaticLookup>::Source>,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::cancel_approval(
                origin,
                class,
                instance,
                maybe_check_delegate,
            )
        }

        /// Alter the attributes of a given asset.
        #[pallet::weight(T::WeightInfo::force_asset_status())]
        #[allow(clippy::too_many_arguments)]
        pub fn force_asset_status(
            origin: OriginFor<T>,
            class: T::ClassId,
            owner: <T::Lookup as StaticLookup>::Source,
            issuer: <T::Lookup as StaticLookup>::Source,
            admin: <T::Lookup as StaticLookup>::Source,
            freezer: <T::Lookup as StaticLookup>::Source,
            free_holding: bool,
            is_frozen: bool,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::force_asset_status(
                origin,
                class,
                owner,
                issuer,
                admin,
                freezer,
                free_holding,
                is_frozen,
            )
        }

        /// Set an attribute for an asset class or instance.
        #[pallet::weight(T::WeightInfo::set_attribute())]
        pub fn set_attribute(
            origin: OriginFor<T>,
            class: T::ClassId,
            maybe_instance: Option<T::InstanceId>,
            key: BoundedVec<u8, T::KeyLimit>,
            value: BoundedVec<u8, T::ValueLimit>,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::set_attribute(origin, class, maybe_instance, key, value)
        }

        /// Set an attribute for an asset class or instance.
        #[pallet::weight(T::WeightInfo::clear_attribute())]
        pub fn clear_attribute(
            origin: OriginFor<T>,
            class: T::ClassId,
            maybe_instance: Option<T::InstanceId>,
            key: BoundedVec<u8, T::KeyLimit>,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::clear_attribute(origin, class, maybe_instance, key)
        }

        /// Set the metadata for an asset instance.
        #[pallet::weight(T::WeightInfo::set_metadata())]
        pub fn set_metadata(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
            data: BoundedVec<u8, T::StringLimit>,
            is_frozen: bool,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::set_metadata(origin, class, instance, data, is_frozen)
        }

        /// Clear the metadata for an asset instance.
        #[pallet::weight(T::WeightInfo::clear_metadata())]
        pub fn clear_metadata(
            origin: OriginFor<T>,
            class: T::ClassId,
            instance: T::InstanceId,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::clear_metadata(origin, class, instance)
        }

        /// Set the metadata for an asset class.
        #[pallet::weight(T::WeightInfo::set_class_metadata())]
        pub fn set_class_metadata(
            origin: OriginFor<T>,
            class: T::ClassId,
            data: BoundedVec<u8, T::StringLimit>,
            is_frozen: bool,
        ) -> DispatchResult {
            pallet_uniques::Pallet::<T>::set_class_metadata(origin, class, data, is_frozen)
        }

        /// Clear the metadata for an asset class.
        #[pallet::weight(T::WeightInfo::clear_class_metadata())]
        pub fn clear_class_metadata(origin: OriginFor<T>, class: T::ClassId) -> DispatchResult {
            pallet_uniques::Pallet::<T>::clear_class_metadata(origin, class)
        }

        #[pallet::weight(T::WeightInfo::create())]
        pub fn deip_create_nft(
            origin: OriginFor<T>,
            class: DeipNftClassIdOf<T>,
            admin: <T as frame_system::Config>::AccountId,
            project_id: Option<DeipProjectIdOf<T>>,
        ) -> DispatchResultWithPostInfo {
            // If project id is provided ensure that admin is in team.
            if let Some(project_id) = project_id.as_ref() {
                todo!()
            }

            // Check if NFT with this deip id exist.
            ensure!(
                !NftClassIdByDeipNftClassId::<T>::contains_key(class),
                Error::<T>::DeipNftClassIdExists
            );

            // Get next origin class id.
            let new_class_id = NextNftClassId::<T>::get();

            // Dispatch call to origin uniques pallet.
            let admin_source = <T::Lookup as StaticLookup>::unlookup(admin);
            let call =
                pallet_uniques::Call::<T>::create { class: new_class_id, admin: admin_source };
            let post_dispatch_info = call.dispatch_bypass_filter(origin)?;

            // Save next class id.
            let next_class_id =
                new_class_id.checked_add(&One::one()).ok_or(Error::<T>::NftClassIdOverflow)?;
            NextNftClassId::<T>::put(next_class_id);

            // Insert id to map.
            NftClassIdByDeipNftClassId::<T>::insert(class, new_class_id);

            // IF project id is provided add id to projects map.
            if let Some(project_id) = project_id {
                todo!()
            }

            Ok(post_dispatch_info)
        }
    }
}