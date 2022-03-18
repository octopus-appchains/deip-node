#![allow(unused_imports)]

use jsonrpc_core::{
    futures::{future, FutureExt, TryFutureExt},
    futures_executor::block_on,
    futures_util::{stream::FuturesOrdered, TryStreamExt},
};
use jsonrpc_derive::rpc;

use std::{iter::FromIterator, vec::Vec};

use codec::{Codec, Decode, Encode, Input};

use sp_runtime::traits::{AtLeast32BitUnsigned, Block as BlockT};

use sp_core::storage::StorageKey;

use frame_support::{Blake2_128Concat, Identity, ReversibleStorageHasher, StorageHasher};

use common_rpc::{
    chain_key_hash_double_map, chain_key_hash_map, get_list_by_keys, get_value, get_value_and_map,
    get_values_paged_and_map, prefix, to_rpc_error, BoxFutureResult, Error, HashOf, HashedKey,
    HashedKeyRef, HashedKeyTrait, ListResult,
};

mod types;
use types::*;

/// Names of pallets in construct_runtime!.
const PARITYTECH_PALLET_UNIQUES: &[u8] = b"ParityTechUniques";
const DEIP_PALLET_UNIQUES: &[u8] = b"Uniques";

#[rpc]
pub trait DeipUniquesRpc<
    BlockHash,
    ClassId,
    InstanceId,
    AccountId,
    DepositBalance,
    Extra,
    DeipClassId,
> where
    ClassId: Encode + Decode,
    InstanceId: Encode + Decode,
    DeipClassId: Encode + Decode,
    AccountId: Decode,
    DepositBalance: Decode + AtLeast32BitUnsigned + Clone,
    Extra: Decode,
{
    /// Get details of class by id.
    #[rpc(name = "uniques_getClass")]
    fn get_class(
        &self,
        at: Option<BlockHash>,
        id: DeipClassId,
    ) -> BoxFutureResult<Option<ClassDetails<AccountId, DepositBalance>>>;

    // #[rpc(name = "uniques_getClassList")]
    // fn get_class_list(
    //     &self,
    //     at: Option<BlockHash>,
    //     count: u32,
    //     start_id: Option<(DeipClassId, ClassId)>,
    // ) -> BoxFutureResult<
    //     Vec<ListResult<(DeipClassId, ClassId), ClassDetails<AccountId, DepositBalance>>>,
    // >;

    // #[rpc(name = "uniques_getClassInstanceList")]
    // fn get_class_instance_list(
    //     &self,
    //     at: Option<BlockHash>,
    //     count: u32,
    //     start_id: Option<(DeipClassId, AccountId)>,
    // ) -> BoxFutureResult<Vec<ClassInstanceWithIds<DeipClassId, InstanceId, AccountId, Extra>>>;

    // #[rpc(name = "uniques_getClassInstanceByOwner")]
    // fn get_class_instance_by_owner(
    //     &self,
    //     at: Option<BlockHash>,
    //     owner: AccountId,
    //     class: DeipClassId,
    // ) -> BoxFutureResult<Option<ClassInstance<InstanceId, Extra>>>;

    // #[rpc(name = "uniques_getClassInstanceListByClass")]
    // fn get_class_instance_list_by_class(
    //     &self,
    //     at: Option<BlockHash>,
    //     class: DeipClassId,
    //     count: u32,
    //     start_id: Option<AccountId>,
    // ) -> BoxFutureResult<Vec<ClassInstanceWithOwner<InstanceId, AccountId, Extra>>>;
}

pub struct DeipUniquesRpcObj<State, B> {
    state: State,
    _marker: std::marker::PhantomData<B>,
}

impl<State, B> DeipUniquesRpcObj<State, B> {
    pub fn new(state: State) -> Self {
        Self { state, _marker: Default::default() }
    }
}

impl<State, Block, ClassId, InstanceId, AccountId, DepositBalance, Extra, DeipClassId>
    DeipUniquesRpc<
        HashOf<Block>,
        ClassId,
        InstanceId,
        AccountId,
        DepositBalance,
        Extra,
        DeipClassId,
    > for DeipUniquesRpcObj<State, Block>
where
    ClassId: 'static + Codec + Send,
    InstanceId: Codec + Send + 'static,
    DeipClassId: 'static + Send + Codec + Clone,
    AccountId: Codec + Send + 'static,
    DepositBalance: 'static + Send + Encode + Decode + AtLeast32BitUnsigned + Clone,
    Extra: 'static + Send + Decode,
    State: sc_rpc_api::state::StateApi<HashOf<Block>>,
    Block: BlockT,
{
    fn get_class(
        &self,
        at: Option<HashOf<Block>>,
        id: DeipClassId,
    ) -> BoxFutureResult<Option<ClassDetails<AccountId, DepositBalance>>> {
        let key_encoded = id.encode();

        let map = |k: StorageKey| {
            let key_hashed = HashedKey::<Blake2_128Concat>::unsafe_from_encoded(&k.0);
            let key = chain_key_hash_map(&prefix(PARITYTECH_PALLET_UNIQUES, b"Class"), &key_hashed);
            self.state
                .storage(key.clone(), at)
                .map_ok(|v| (v, key))
                .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
        };

        let index_prefix = prefix(DEIP_PALLET_UNIQUES, b"NftClassIdByDeipNftClassId");
        let index_key = HashedKey::<Identity>::unsafe_from_encoded(&key_encoded);

        let prefix_key = chain_key_hash_map(&index_prefix, &index_key);
        get_value_and_map::<
            ClassKeyValue<ClassId, AccountId, DepositBalance>,
            Identity,
            _,
            _,
            _,
            _,
            _,
        >(&self.state, at, prefix_key, map)
        .map_ok(|v| v.map(|item| item.value))
        .boxed()
    }

    // fn get_class_list(
    //     &self,
    //     at: Option<HashOf<Block>>,
    //     count: u32,
    //     start_id: Option<(DeipClassId, ClassId)>,
    // ) -> BoxFutureResult<
    //     Vec<ListResult<(DeipClassId, ClassId), ClassDetails<AccountId, DepositBalance>>>,
    // > {
    //     let index_prefix = prefix(DEIP_PALLET_UNIQUES, b"NftClassIdByDeipNftClassId");
    //     let start_key = start_id.map(|(index_id, id)| {
    //         let key_first = HashedKey::<Identity>::new(&index_id);
    //         let key_second = HashedKey::<Blake2_128Concat>::new(&id);
    //         chain_key_hash_double_map(&index_prefix, &key_first, &key_second)
    //     });

    //     let map = |k: StorageKey| -> BoxFutureResult<(
    //         Option<common_rpc::StorageData>,
    //         StorageKey,
    //         DeipClassId,
    //     )> {
    //         // below we retrieve key in the other map from the index map key
    //         let no_prefix = Identity::reverse(&k.0[32..]);
    //         // decode DeipClassId and save the length of processed bytes
    //         let input = &mut &*no_prefix;
    //         let index_key = match DeipClassId::decode(input) {
    //             Ok(k) => k,
    //             Err(e) => {
    //                 let data = format!("{:?}: {}", no_prefix, e);
    //                 let rpc_error = to_rpc_error(Error::DeipClassIdDecodeFailed, Some(data));
    //                 return future::err(rpc_error).boxed()
    //             },
    //         };

    //         let len = match Input::remaining_len(input).ok().flatten() {
    //             Some(l) => l,
    //             None => {
    //                 let data = format!("{:?}", input);
    //                 let rpc_err = to_rpc_error(Error::DeipClassIdRemainingLengthFailed, Some(data));
    //                 return future::err(rpc_err).boxed()
    //             },
    //         };

    //         let key_hashed =
    //             HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(&no_prefix[len..]);

    //         let key = chain_key_hash_map(&prefix(PARITYTECH_PALLET_UNIQUES, b"Class"), &key_hashed);

    //         self.state
    //             .storage(key.clone(), at)
    //             .map_ok(|v| (v, key, index_key))
    //             .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
    //             .boxed()
    //     };

    //     get_values_paged_and_map::<
    //         ClassKeyValue<ClassId, AccountId, DepositBalance>,
    //         Blake2_128Concat,
    //         _,
    //         _,
    //         _,
    //         _,
    //         _,
    //     >(&self.state, at, StorageKey(index_prefix), count, start_key, map)
    // }

    // fn get_class_instance_list(
    //     &self,
    //     at: Option<HashOf<Block>>,
    //     count: u32,
    //     start_id: Option<(DeipClassId, AccountId)>,
    // ) -> BoxFutureResult<Vec<ClassInstanceWithIds<DeipClassId, InstanceId, AccountId, Extra>>> {
    //     let storage_prefix = prefix(PARITYTECH_PALLET_UNIQUES, b"Account");

    //     let fut = async {
    //         let start_key = match start_id {
    //             None => None,
    //             Some((class, account)) => {
    //                 let index_hashed = HashedKey::<Identity>::new(&class);
    //                 let storage_prefix = prefix(DEIP_PALLET_UNIQUES, b"NftClassIdByDeipNftClassId");
    //                 let prefix_key = chain_key_hash_map(&storage_prefix, &index_hashed);
    //                 let mut keys = self
    //                     .state
    //                     .storage_keys_paged(Some(prefix_key), 1, None, at)
    //                     .await
    //                     .map_err(|e| {
    //                         let data = format!("{:?}", e);
    //                         to_rpc_error(Error::ScRpcApiError, Some(data))
    //                     })?;
    //                 if keys.is_empty() {
    //                     return Ok(vec![])
    //                 }

    //                 let index_key = keys.pop().unwrap();
    //                 let no_prefix = &index_key.0[32..];
    //                 let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
    //                     &no_prefix[index_hashed.as_ref().len()..],
    //                 );

    //                 Some(chain_key_hash_double_map(
    //                     &storage_prefix,
    //                     &key_hashed,
    //                     &HashedKey::<Blake2_128Concat>::new(&account),
    //                 ))
    //             },
    //         };

    //         let state = &self.state;
    //         let keys = state
    //             .storage_keys_paged(Some(StorageKey(storage_prefix)), count, start_key, at)
    //             .await
    //             .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))?;
    //         if keys.is_empty() {
    //             return Ok(vec![])
    //         }

    //         let keys: Vec<_> = FuturesOrdered::from_iter(keys.into_iter().map(|k| async {
    //             // we have to wait for data so another request to
    //             // index 1-to-1 map can be made
    //             let storage_data = state
    //                 .storage(k.clone(), at)
    //                 .await
    //                 .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))?;

    //             let no_prefix = &k.0[32..];
    //             let len = no_prefix.len();
    //             let no_prefix_no_hash = &mut Blake2_128Concat::reverse(no_prefix);

    //             ClassId::skip(no_prefix_no_hash).map_err(|e| {
    //                 to_rpc_error(Error::ClassIdDecodeFailed, Some(format!("{:?}", e)))
    //             })?;
    //             let remaining_len =
    //                 Input::remaining_len(no_prefix_no_hash).ok().flatten().ok_or_else(|| {
    //                     let data = Some(format!("{:?}", no_prefix_no_hash));
    //                     to_rpc_error(Error::ClassIdRemainingLengthFailed, data)
    //                 })?;

    //             let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
    //                 &no_prefix[..len - remaining_len],
    //             );
    //             let storage_prefix = prefix(DEIP_PALLET_UNIQUES, b"DeipClassIdByClassId");
    //             let prefix_key = chain_key_hash_map(&storage_prefix, &key_hashed);
    //             state
    //                 .storage_keys_paged(Some(prefix_key), 1, None, at)
    //                 .await
    //                 .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
    //                 .map(|mut index_keys| (index_keys.pop(), k, storage_data))
    //         }))
    //         .try_collect()
    //         .await?;

    //         let result = Vec::with_capacity(keys.len());
    //         keys.into_iter().try_fold(result, |mut result, kv| {
    //             let (index_key, key, value) = kv;
    //             let data = match value {
    //                 None => return Ok(result),
    //                 Some(d) => d,
    //             };

    //             let index_key = if let Some(key) = index_key {
    //                 key
    //             } else {
    //                 return Err(to_rpc_error(Error::DeipClassIdInverseIndexFailed, None))
    //             };

    //             let no_prefix = &index_key.0[32..];
    //             let len = no_prefix.len();
    //             let no_prefix_no_hash = &mut Blake2_128Concat::reverse(no_prefix);

    //             if let Err(e) = ClassId::skip(no_prefix_no_hash) {
    //                 return Err(to_rpc_error(
    //                     Error::ClassIdDecodeFailed,
    //                     Some(format!("{:?}: {}", &index_key.0, e)),
    //                 ))
    //             }
    //             let remaining_len = match Input::remaining_len(no_prefix_no_hash).ok().flatten() {
    //                 Some(l) => l,
    //                 None =>
    //                     return Err(to_rpc_error(
    //                         Error::ClassIdRemainingLengthFailed,
    //                         Some(format!("{:?}", no_prefix_no_hash)),
    //                     )),
    //             };

    //             let no_prefix = Identity::reverse(&no_prefix[len - remaining_len..]);
    //             let class = match DeipClassId::decode(&mut &*no_prefix) {
    //                 Err(e) =>
    //                     return Err(to_rpc_error(
    //                         Error::DeipClassIdDecodeFailed,
    //                         Some(format!("{:?}: {}", &key.0, e)),
    //                     )),
    //                 Ok(id) => id,
    //             };

    //             let no_prefix = &key.0[32..];
    //             let len = no_prefix.len();
    //             let no_prefix_no_hash = &mut Blake2_128Concat::reverse(no_prefix);

    //             match ClassId::skip(no_prefix_no_hash) {
    //                 Ok(_) => (),
    //                 Err(_) =>
    //                     return Err(to_rpc_error(
    //                         Error::ClassIdDecodeFailed,
    //                         Some(format!("{:?}", &key.0)),
    //                     )),
    //             };
    //             let remaining_len = match Input::remaining_len(no_prefix_no_hash).ok().flatten() {
    //                 Some(l) => l,
    //                 None =>
    //                     return Err(to_rpc_error(
    //                         Error::ClassIdRemainingLengthFailed,
    //                         Some(format!("{:?}", no_prefix_no_hash)),
    //                     )),
    //             };

    //             let no_prefix = Blake2_128Concat::reverse(&no_prefix[len - remaining_len..]);
    //             let account = match AccountId::decode(&mut &*no_prefix) {
    //                 Err(_) =>
    //                     return Err(to_rpc_error(
    //                         Error::AccountIdDecodeFailed,
    //                         Some(format!("{:?}", &key.0)),
    //                     )),
    //                 Ok(id) => id,
    //             };

    //             match ClassInstance::<InstanceId, Extra>::decode(&mut &data.0[..]) {
    //                 Err(_) => Err(to_rpc_error(
    //                     Error::ClassInstanceDecodeFailed,
    //                     Some(format!("{:?}", data)),
    //                 )),
    //                 Ok(balance) => {
    //                     result.push(ClassInstanceWithIds { class, account, balance });
    //                     Ok(result)
    //                 },
    //             }
    //         })
    //     };
    //     future::ready(block_on(fut)).boxed() //@TODO remove block_on
    // }

    // fn get_class_instance_by_owner(
    //     &self,
    //     at: Option<HashOf<Block>>,
    //     owner: AccountId,
    //     class: DeipClassId,
    // ) -> BoxFutureResult<Option<ClassInstance<InstanceId, Extra>>> {
    //     let index_hashed = HashedKey::<Identity>::new(&class);
    //     let storage_prefix = prefix(DEIP_PALLET_UNIQUES, b"ClassIdByDeipClassId");
    //     let prefix_key = chain_key_hash_map(&storage_prefix, &index_hashed);
    //     let mut keys = match block_on(self.state.storage_keys_paged(Some(prefix_key), 1, None, at))
    //     {
    //         Ok(k) => k,
    //         Err(e) =>
    //             return future::err(to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
    //                 .boxed(),
    //     };
    //     if keys.is_empty() {
    //         return future::ok(None).boxed()
    //     }

    //     let key = keys.pop().unwrap();

    //     let no_prefix = &key.0[32..];
    //     let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
    //         &no_prefix[index_hashed.as_ref().len()..],
    //     );

    //     let storage_prefix = prefix(PARITYTECH_PALLET_UNIQUES, b"Account");
    //     let key_second = HashedKey::<Blake2_128Concat>::new(&owner);
    //     let key = chain_key_hash_double_map(&storage_prefix, &key_hashed, &key_second);
    //     get_value(&self.state, key, at)
    // }

    // fn get_class_instance_list_by_class(
    //     &self,
    //     at: Option<HashOf<Block>>,
    //     class: DeipClassId,
    //     count: u32,
    //     start_id: Option<AccountId>,
    // ) -> BoxFutureResult<Vec<ClassInstanceWithOwner<InstanceId, AccountId, Extra>>> {
    //     // work with index
    //     let index_hashed = HashedKey::<Identity>::new(&class);
    //     let prefix_key = chain_key_hash_map(
    //         &prefix(DEIP_PALLET_UNIQUES, b"ClassIdByDeipClassId"),
    //         &index_hashed,
    //     );
    //     let mut keys = match block_on(self.state.storage_keys_paged(Some(prefix_key), 1, None, at))
    //     {
    //         Ok(k) => k,
    //         Err(e) =>
    //             return future::err(to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
    //                 .boxed(),
    //     };
    //     if keys.is_empty() {
    //         return future::ok(vec![]).boxed()
    //     }

    //     let key = keys.pop().unwrap();

    //     let no_prefix = &key.0[32..];
    //     let len = index_hashed.as_ref().len();
    //     let class_encoded = Blake2_128Concat::reverse(&no_prefix[len..]);
    //     let class_encoded_size = class_encoded.len();
    //     let class_hashed =
    //         HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(&no_prefix[len..]);

    //     let prefix = prefix(PARITYTECH_PALLET_UNIQUES, b"Account");

    //     let start_key = start_id.map(|account_id| {
    //         StorageKey(
    //             prefix
    //                 .iter()
    //                 .chain(class_hashed.as_ref())
    //                 .chain(&account_id.using_encoded(Blake2_128Concat::hash))
    //                 .copied()
    //                 .collect(),
    //         )
    //     });

    //     let prefix = prefix.iter().chain(class_hashed.as_ref()).copied().collect();

    //     let state = &self.state;
    //     let keys = match block_on(state.storage_keys_paged(
    //         Some(StorageKey(prefix)),
    //         count,
    //         start_key,
    //         at,
    //     )) {
    //         Ok(k) => k,
    //         Err(e) =>
    //             return future::err(to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
    //                 .boxed(),
    //     };
    //     if keys.is_empty() {
    //         return future::ok(vec![]).boxed()
    //     }

    //     let key_futures: FuturesOrdered<_> = keys
    //         .into_iter()
    //         .map(|k| {
    //             state
    //                 .storage(k.clone(), at)
    //                 .map_ok(|v| (k, v))
    //                 .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
    //         })
    //         .collect();

    //     let result = Vec::with_capacity(key_futures.len());
    //     key_futures
    //         .try_fold(result, move |mut result, kv| {
    //             let (key, value) = kv;
    //             let data = match value {
    //                 None => return future::ok(result),
    //                 Some(d) => d,
    //             };

    //             let no_prefix = Blake2_128Concat::reverse(&key.0[32..]);
    //             let no_prefix = Blake2_128Concat::reverse(&no_prefix[class_encoded_size..]);
    //             let account = match AccountId::decode(&mut &*no_prefix) {
    //                 Err(_) =>
    //                     return future::err(to_rpc_error(
    //                         Error::AccountIdDecodeFailed,
    //                         Some(format!("{:?}", &key.0)),
    //                     )),
    //                 Ok(id) => id,
    //             };

    //             match ClassInstance::<InstanceId, Extra>::decode(&mut &data.0[..]) {
    //                 Err(_) => future::err(to_rpc_error(
    //                     Error::ClassInstanceDecodeFailed,
    //                     Some(format!("{:?}", data)),
    //                 )),
    //                 Ok(balance) => {
    //                     result.push(ClassInstanceWithOwner { account, balance });
    //                     future::ok(result)
    //                 },
    //             }
    //         })
    //         .boxed()
    // }
}
