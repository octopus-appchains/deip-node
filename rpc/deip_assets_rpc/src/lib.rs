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
    chain_key_hash_double_map, chain_key_hash_map, get_list_by_keys, get_value, prefix,
    to_rpc_error, BoxFutureResult, Error, HashOf, HashedKey, HashedKeyRef, HashedKeyTrait,
    ListResult,
};

mod types;
use types::*;

/// Names of pallets in construct_runtime!.
const PARITYTECH_PALLET_ASSETS: &[u8] = b"ParityTechAssets";
const DEIP_PALLET_ASSETS: &[u8] = b"Assets";

#[rpc]
pub trait DeipAssetsRpc<BlockHash, AssetId, Balance, AccountId, DepositBalance, Extra, DeipAssetId>
where
    AssetId: Encode + Decode,
    DeipAssetId: Encode + Decode,
    Balance: Decode + AtLeast32BitUnsigned + Clone,
    AccountId: Decode,
    DepositBalance: Decode + AtLeast32BitUnsigned + Clone,
    Extra: Decode,
{
    #[rpc(name = "assets_getAsset")]
    fn get_asset(
        &self,
        at: Option<BlockHash>,
        id: DeipAssetId,
    ) -> BoxFutureResult<Option<AssetDetails<Balance, AccountId, DepositBalance>>>;

    #[rpc(name = "assets_getAssetList")]
    fn get_asset_list(
        &self,
        at: Option<BlockHash>,
        count: u32,
        start_id: Option<(DeipAssetId, AssetId)>,
    ) -> BoxFutureResult<
        Vec<ListResult<(DeipAssetId, AssetId), AssetDetails<Balance, AccountId, DepositBalance>>>,
    >;

    #[rpc(name = "assets_getAssetBalanceList")]
    fn get_asset_balance_list(
        &self,
        at: Option<BlockHash>,
        count: u32,
        start_id: Option<(DeipAssetId, AccountId)>,
    ) -> BoxFutureResult<Vec<AssetBalanceWithIds<DeipAssetId, Balance, AccountId, Extra>>>;

    #[rpc(name = "assets_getAssetBalanceByOwner")]
    fn get_asset_balance_by_owner(
        &self,
        at: Option<BlockHash>,
        owner: AccountId,
        asset: DeipAssetId,
    ) -> BoxFutureResult<Option<AssetBalance<Balance, Extra>>>;

    #[rpc(name = "assets_getAssetBalanceListByAsset")]
    fn get_asset_balance_list_by_asset(
        &self,
        at: Option<BlockHash>,
        asset: DeipAssetId,
        count: u32,
        start_id: Option<AccountId>,
    ) -> BoxFutureResult<Vec<AssetBalanceWithOwner<Balance, AccountId, Extra>>>;
}

pub struct DeipAssetsRpcObj<State, B> {
    state: State,
    _marker: std::marker::PhantomData<B>,
}

impl<State, B> DeipAssetsRpcObj<State, B> {
    pub fn new(state: State) -> Self {
        Self { state, _marker: Default::default() }
    }
}

impl<State, Block, AssetId, Balance, AccountId, DepositBalance, Extra, DeipAssetId>
    DeipAssetsRpc<HashOf<Block>, AssetId, Balance, AccountId, DepositBalance, Extra, DeipAssetId>
    for DeipAssetsRpcObj<State, Block>
where
    AssetId: 'static + Codec + Send,
    DeipAssetId: 'static + Send + Codec + Clone,
    Balance: 'static + Decode + AtLeast32BitUnsigned + Clone + Send,
    AccountId: 'static + Codec + Send,
    DepositBalance: 'static + Send + Encode + Decode + AtLeast32BitUnsigned + Clone,
    Extra: 'static + Send + Decode,
    State: sc_rpc_api::state::StateApi<HashOf<Block>>,
    Block: BlockT,
{
    fn get_asset(
        &self,
        at: Option<HashOf<Block>>,
        id: DeipAssetId,
    ) -> BoxFutureResult<Option<AssetDetails<Balance, AccountId, DepositBalance>>> {
        let key_encoded = id.encode();
        let key_encoded_size = key_encoded.len();

        let map = |k: StorageKey| {
            // below we retrieve key in the other map from the index map key
            let no_prefix = Identity::reverse(&k.0[32..]);
            let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
                &no_prefix[key_encoded_size..],
            );

            let key = chain_key_hash_map(&prefix(PARITYTECH_PALLET_ASSETS, b"Asset"), &key_hashed);

            self.state
                .storage(key.clone(), at)
                .map_ok(|v| (v, key))
                .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
        };

        let index_prefix = prefix(DEIP_PALLET_ASSETS, b"AssetIdByDeipAssetId");
        let index_key = HashedKey::<Identity>::unsafe_from_encoded(&key_encoded);

        let prefix_key = chain_key_hash_map(&index_prefix, &index_key);
        get_list_by_keys::<
            types::AssetKeyValue<AssetId, Balance, AccountId, DepositBalance>,
            Identity,
            _,
            _,
            _,
            _,
            _,
        >(&self.state, at, prefix_key, 1, None, map)
        .map_ok(|mut v| v.pop().map(|item| item.value))
        .boxed()
    }

    fn get_asset_list(
        &self,
        at: Option<HashOf<Block>>,
        count: u32,
        start_id: Option<(DeipAssetId, AssetId)>,
    ) -> BoxFutureResult<
        Vec<ListResult<(DeipAssetId, AssetId), AssetDetails<Balance, AccountId, DepositBalance>>>,
    > {
        let index_prefix = prefix(DEIP_PALLET_ASSETS, b"AssetIdByDeipAssetId");
        let start_key = start_id.map(|(index_id, id)| {
            chain_key_hash_double_map(
                &index_prefix,
                &HashedKey::<Identity>::new(&index_id),
                &HashedKey::<Blake2_128Concat>::new(&id),
            )
        });

        let map = |k: StorageKey| -> BoxFutureResult<(
            Option<common_rpc::StorageData>,
            StorageKey,
            DeipAssetId,
        )> {
            // below we retrieve key in the other map from the index map key
            let no_prefix = Identity::reverse(&k.0[32..]);
            // decode DeipAssetId and save the length of processed bytes
            let input = &mut &*no_prefix;
            let index_key = match DeipAssetId::decode(input) {
                Ok(k) => k,
                Err(_) => {
                    let rpc_error = to_rpc_error(
                        Error::DeipAssetIdDecodeFailed,
                        Some(format!("{:?}", no_prefix)),
                    );
                    return future::err(rpc_error).boxed()
                },
            };

            let len = match Input::remaining_len(input).ok().flatten() {
                Some(l) => l,
                None =>
                    return future::err(to_rpc_error(
                        Error::DeipAssetIdRemainingLengthFailed,
                        Some(format!("{:?}", input)),
                    ))
                    .boxed(),
            };

            let key_hashed =
                HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(&no_prefix[len..]);

            let key = chain_key_hash_map(&prefix(PARITYTECH_PALLET_ASSETS, b"Asset"), &key_hashed);

            self.state
                .storage(key.clone(), at)
                .map_ok(|v| (v, key, index_key))
                .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
                .boxed()
        };

        get_list_by_keys::<
            types::AssetKeyValue<AssetId, Balance, AccountId, DepositBalance>,
            Blake2_128Concat,
            _,
            _,
            _,
            _,
            _,
        >(&self.state, at, StorageKey(index_prefix), count, start_key, map)
    }

    fn get_asset_balance_list(
        &self,
        at: Option<HashOf<Block>>,
        count: u32,
        start_id: Option<(DeipAssetId, AccountId)>,
    ) -> BoxFutureResult<Vec<AssetBalanceWithIds<DeipAssetId, Balance, AccountId, Extra>>> {
        let prefix = prefix(PARITYTECH_PALLET_ASSETS, b"Account");

        let fut = async {
            let start_key = match start_id {
                None => None,
                Some((asset, account)) => {
                    let index_hashed = HashedKey::<Identity>::new(&asset);
                    let prefix_key = chain_key_hash_map(
                        &crate::prefix(DEIP_PALLET_ASSETS, b"AssetIdByDeipAssetId"),
                        &index_hashed,
                    );
                    let mut keys = self
                        .state
                        .storage_keys_paged(Some(prefix_key), 1, None, at)
                        .await
                        .map_err(|e| {
                            let data = format!("{:?}", e);
                            to_rpc_error(Error::ScRpcApiError, Some(data))
                        })?;
                    if keys.is_empty() {
                        return Ok(vec![])
                    }

                    let index_key = keys.pop().unwrap();
                    let no_prefix = &index_key.0[32..];
                    let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
                        &no_prefix[index_hashed.as_ref().len()..],
                    );

                    Some(chain_key_hash_double_map(
                        &prefix,
                        &key_hashed,
                        &HashedKey::<Blake2_128Concat>::new(&account),
                    ))
                },
            };

            let state = &self.state;
            let keys = state
                .storage_keys_paged(Some(StorageKey(prefix)), count, start_key, at)
                .await
                .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))?;
            if keys.is_empty() {
                return Ok(vec![])
            }

            let keys: Vec<_> = FuturesOrdered::from_iter(keys.into_iter().map(|k| async {
                // we have to wait for data so another request to
                // index 1-to-1 map can be made
                let storage_data = state
                    .storage(k.clone(), at)
                    .await
                    .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))?;

                let no_prefix = &k.0[32..];
                let len = no_prefix.len();
                let no_prefix_no_hash = &mut Blake2_128Concat::reverse(no_prefix);

                AssetId::skip(no_prefix_no_hash).map_err(|e| {
                    to_rpc_error(Error::AssetIdDecodeFailed, Some(format!("{:?}", e)))
                })?;
                let remaining_len =
                    Input::remaining_len(no_prefix_no_hash).ok().flatten().ok_or_else(|| {
                        to_rpc_error(
                            Error::AssetIdRemainingLengthFailed,
                            Some(format!("{:?}", no_prefix_no_hash)),
                        )
                    })?;

                let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
                    &no_prefix[..len - remaining_len],
                );
                let prefix_key = chain_key_hash_map(
                    &crate::prefix(DEIP_PALLET_ASSETS, b"DeipAssetIdByAssetId"),
                    &key_hashed,
                );
                state
                    .storage_keys_paged(Some(prefix_key), 1, None, at)
                    .await
                    .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
                    .map(|mut index_keys| (index_keys.pop(), k, storage_data))
            }))
            .try_collect()
            .await?;

            let result = Vec::with_capacity(keys.len());
            keys.into_iter().try_fold(result, |mut result, kv| {
                let (index_key, key, value) = kv;
                let data = match value {
                    None => return Ok(result),
                    Some(d) => d,
                };

                let index_key = match index_key {
                    Some(k) => k,
                    None => return Err(to_rpc_error(Error::DeipAssetIdInverseIndexFailed, None)),
                };

                let no_prefix = &index_key.0[32..];
                let len = no_prefix.len();
                let no_prefix_no_hash = &mut Blake2_128Concat::reverse(no_prefix);

                match AssetId::skip(no_prefix_no_hash) {
                    Ok(_) => (),
                    Err(_) =>
                        return Err(to_rpc_error(
                            Error::AssetIdDecodeFailed,
                            Some(format!("{:?}", &index_key.0)),
                        )),
                };
                let remaining_len = match Input::remaining_len(no_prefix_no_hash).ok().flatten() {
                    Some(l) => l,
                    None =>
                        return Err(to_rpc_error(
                            Error::AssetIdRemainingLengthFailed,
                            Some(format!("{:?}", no_prefix_no_hash)),
                        )),
                };

                let no_prefix = Identity::reverse(&no_prefix[len - remaining_len..]);
                let asset = match DeipAssetId::decode(&mut &*no_prefix) {
                    Err(_) =>
                        return Err(to_rpc_error(
                            Error::DeipAssetIdDecodeFailed,
                            Some(format!("{:?}", &key.0)),
                        )),
                    Ok(id) => id,
                };

                let no_prefix = &key.0[32..];
                let len = no_prefix.len();
                let no_prefix_no_hash = &mut Blake2_128Concat::reverse(no_prefix);

                match AssetId::skip(no_prefix_no_hash) {
                    Ok(_) => (),
                    Err(_) =>
                        return Err(to_rpc_error(
                            Error::AssetIdDecodeFailed,
                            Some(format!("{:?}", &key.0)),
                        )),
                };
                let remaining_len = match Input::remaining_len(no_prefix_no_hash).ok().flatten() {
                    Some(l) => l,
                    None =>
                        return Err(to_rpc_error(
                            Error::AssetIdRemainingLengthFailed,
                            Some(format!("{:?}", no_prefix_no_hash)),
                        )),
                };

                let no_prefix = Blake2_128Concat::reverse(&no_prefix[len - remaining_len..]);
                let account = match AccountId::decode(&mut &*no_prefix) {
                    Err(_) =>
                        return Err(to_rpc_error(
                            Error::AccountIdDecodeFailed,
                            Some(format!("{:?}", &key.0)),
                        )),
                    Ok(id) => id,
                };

                match AssetBalance::<Balance, Extra>::decode(&mut &data.0[..]) {
                    Err(_) => Err(to_rpc_error(
                        Error::AssetBalanceDecodeFailed,
                        Some(format!("{:?}", data)),
                    )),
                    Ok(balance) => {
                        result.push(AssetBalanceWithIds { asset, account, balance });
                        Ok(result)
                    },
                }
            })
        };
        let res = block_on(fut); //@TODO remove block_on
        future::ready(res).boxed()
    }

    fn get_asset_balance_by_owner(
        &self,
        at: Option<HashOf<Block>>,
        owner: AccountId,
        asset: DeipAssetId,
    ) -> BoxFutureResult<Option<AssetBalance<Balance, Extra>>> {
        let index_hashed = HashedKey::<Identity>::new(&asset);
        let prefix_key =
            chain_key_hash_map(&prefix(DEIP_PALLET_ASSETS, b"AssetIdByDeipAssetId"), &index_hashed);
        let mut keys = match block_on(self.state.storage_keys_paged(Some(prefix_key), 1, None, at))
        {
            Ok(k) => k,
            Err(e) =>
                return future::err(to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
                    .boxed(),
        };
        if keys.is_empty() {
            return future::ok(None).boxed()
        }

        let key = keys.pop().unwrap();

        let no_prefix = &key.0[32..];
        let key_hashed = HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(
            &no_prefix[index_hashed.as_ref().len()..],
        );

        get_value(
            &self.state,
            chain_key_hash_double_map(
                &prefix(PARITYTECH_PALLET_ASSETS, b"Account"),
                &key_hashed,
                &HashedKey::<Blake2_128Concat>::new(&owner),
            ),
            at,
        )
    }

    fn get_asset_balance_list_by_asset(
        &self,
        at: Option<HashOf<Block>>,
        asset: DeipAssetId,
        count: u32,
        start_id: Option<AccountId>,
    ) -> BoxFutureResult<Vec<AssetBalanceWithOwner<Balance, AccountId, Extra>>> {
        // work with index
        let index_hashed = HashedKey::<Identity>::new(&asset);
        let prefix_key =
            chain_key_hash_map(&prefix(DEIP_PALLET_ASSETS, b"AssetIdByDeipAssetId"), &index_hashed);
        let mut keys = match block_on(self.state.storage_keys_paged(Some(prefix_key), 1, None, at))
        {
            Ok(k) => k,
            Err(e) =>
                return future::err(to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
                    .boxed(),
        };
        if keys.is_empty() {
            return future::ok(vec![]).boxed()
        }

        let key = keys.pop().unwrap();

        let no_prefix = &key.0[32..];
        let len = index_hashed.as_ref().len();
        let asset_encoded = Blake2_128Concat::reverse(&no_prefix[len..]);
        let asset_encoded_size = asset_encoded.len();
        let asset_hashed =
            HashedKeyRef::<'_, Blake2_128Concat>::unsafe_from_hashed(&no_prefix[len..]);

        let prefix = prefix(PARITYTECH_PALLET_ASSETS, b"Account");

        let start_key = start_id.map(|account_id| {
            StorageKey(
                prefix
                    .iter()
                    .chain(asset_hashed.as_ref())
                    .chain(&account_id.using_encoded(Blake2_128Concat::hash))
                    .copied()
                    .collect(),
            )
        });

        let prefix = prefix.iter().chain(asset_hashed.as_ref()).copied().collect();

        let state = &self.state;
        let keys = match block_on(state.storage_keys_paged(
            Some(StorageKey(prefix)),
            count,
            start_key,
            at,
        )) {
            Ok(k) => k,
            Err(e) =>
                return future::err(to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
                    .boxed(),
        };
        if keys.is_empty() {
            return future::ok(vec![]).boxed()
        }

        let key_futures: FuturesOrdered<_> = keys
            .into_iter()
            .map(|k| {
                state
                    .storage(k.clone(), at)
                    .map_ok(|v| (k, v))
                    .map_err(|e| to_rpc_error(Error::ScRpcApiError, Some(format!("{:?}", e))))
            })
            .collect();

        let result = Vec::with_capacity(key_futures.len());
        key_futures
            .try_fold(result, move |mut result, kv| {
                let (key, value) = kv;
                let data = match value {
                    None => return future::ok(result),
                    Some(d) => d,
                };

                let no_prefix = Blake2_128Concat::reverse(&key.0[32..]);
                let no_prefix = Blake2_128Concat::reverse(&no_prefix[asset_encoded_size..]);
                let account = match AccountId::decode(&mut &*no_prefix) {
                    Err(_) =>
                        return future::err(to_rpc_error(
                            Error::AccountIdDecodeFailed,
                            Some(format!("{:?}", &key.0)),
                        )),
                    Ok(id) => id,
                };

                match AssetBalance::<Balance, Extra>::decode(&mut &data.0[..]) {
                    Err(_) => future::err(to_rpc_error(
                        Error::AssetBalanceDecodeFailed,
                        Some(format!("{:?}", data)),
                    )),
                    Ok(balance) => {
                        result.push(AssetBalanceWithOwner { account, balance });
                        future::ok(result)
                    },
                }
            })
            .boxed()
    }
}
