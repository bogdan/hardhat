use std::sync::Arc;

use napi::{
    bindgen_prelude::{BigInt, Buffer, Env},
    tokio::{runtime::Runtime, sync::RwLock},
    JsObject, Status,
};
use napi_derive::napi;
use rethnet_eth::{block::Header, Address, U256};
use rethnet_evm::{
    blockchain::{BlockchainError, SyncBlockchain},
    state::{StateError, SyncState},
    trace::TraceCollector,
    BlockTransactionError, CfgEnv, InvalidTransaction, SyncInspector,
};

use crate::{
    blockchain::Blockchain,
    cast::TryCast,
    config::ConfigOptions,
    context::RethnetContext,
    state::StateManager,
    transaction::{result::TransactionResult, PendingTransaction},
};

use super::{Block, BlockHeader, BlockOptions};

#[napi]
pub struct BlockBuilder {
    builder: Arc<RwLock<Option<rethnet_evm::BlockBuilder>>>,
    blockchain: Arc<RwLock<dyn SyncBlockchain<BlockchainError>>>,
    state: Arc<RwLock<dyn SyncState<StateError>>>,
    runtime: Arc<Runtime>,
}

#[napi]
impl BlockBuilder {
    // TODO: There seems to be a limitation in napi-rs that prevents us from creating
    // a #[napi(factory)] with an async fn
    #[napi(ts_return_type = "Promise<BlockBuilder>")]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub fn create(
        env: Env,
        context: &RethnetContext,
        blockchain: &Blockchain,
        state_manager: &StateManager,
        config: ConfigOptions,
        parent: BlockHeader,
        block: BlockOptions,
    ) -> napi::Result<JsObject> {
        let parent = Header::try_from(parent)?;
        let block = rethnet_eth::block::BlockOptions::try_from(block)?;

        let config = CfgEnv::try_from(config)?;
        let blockchain = (*blockchain).clone();
        let state = (*state_manager).clone();

        let runtime = context.runtime().clone();

        let (deferred, promise) = env.create_deferred()?;
        context.runtime().spawn(async move {
            let result = rethnet_evm::BlockBuilder::new(
                &mut *state.clone().write().await,
                config,
                &parent,
                block,
            )
            .map_or_else(
                |e| Err(napi::Error::new(Status::GenericFailure, e.to_string())),
                |builder| {
                    Ok(Self {
                        builder: Arc::new(RwLock::new(Some(builder))),
                        blockchain,
                        state,
                        runtime,
                    })
                },
            );

            deferred.resolve(|_| result);
        });

        Ok(promise)
    }

    /// Retrieves the amount of gas used by the builder.
    #[napi(getter)]
    pub async fn gas_used(&self) -> napi::Result<BigInt> {
        let builder = self.builder.read().await;
        if let Some(builder) = builder.as_ref() {
            Ok(BigInt {
                sign_bit: false,
                words: builder.gas_used().as_limbs().to_vec(),
            })
        } else {
            Err(napi::Error::new(
                Status::InvalidArg,
                "`this` has been moved in Rust".to_owned(),
            ))
        }
    }

    #[napi]
    #[napi(ts_return_type = "Promise<TransactionResult>")]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub fn add_transaction(
        &self,
        env: Env,
        transaction: &PendingTransaction,
        with_trace: bool,
    ) -> napi::Result<JsObject> {
        let builder = self.builder.clone();
        let transaction = (*transaction).clone();

        let blockchain = self.blockchain.clone();
        let state = self.state.clone();

        let (deferred, promise) = env.create_deferred()?;
        self.runtime.spawn(async move {
            let mut builder = builder.write().await;
            let result = if let Some(builder) = builder.as_mut() {
                let mut tracer = TraceCollector::default();
                let inspector: Option<&mut dyn SyncInspector<BlockchainError, StateError>> =
                    if with_trace { Some(&mut tracer) } else { None };

                builder
                    .add_transaction(&mut *blockchain.write().await, &mut *state.write().await, transaction, inspector)
                    .map_or_else(
                        |e| Err(napi::Error::new(Status::GenericFailure, match e {
                            BlockTransactionError::InvalidTransaction(
                                InvalidTransaction::LackOfFundForMaxFee { fee, balance },
                            ) => format!("sender doesn't have enough funds to send tx. The max upfront cost is: {fee} and the sender's account only has: {balance}"),
                            e => e.to_string(),
                        })),
                        |result| {
                            let trace = if with_trace {
                                Some(tracer.into_trace())
                            } else {
                                None
                            };

                            Ok(TransactionResult::new(result, None, trace))
                        },
                    )
            } else {
                Err(napi::Error::new(
                    Status::InvalidArg,
                    "`this` has been moved in Rust".to_owned(),
                ))
            };

            deferred.resolve(|_| result);
        });

        Ok(promise)
    }

    /// This call consumes the [`BlockBuilder`] object in Rust. Afterwards, you can no longer call
    /// methods on the JS object.
    #[napi]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn finalize(
        &self,
        rewards: Vec<(Buffer, BigInt)>,
        timestamp: Option<BigInt>,
    ) -> napi::Result<Block> {
        let mut builder = self.builder.write().await;
        if let Some(builder) = builder.take() {
            let rewards = rewards
                .into_iter()
                .map(|(address, reward)| {
                    TryCast::<U256>::try_cast(reward)
                        .map(|reward| (Address::from_slice(&address), reward))
                })
                .collect::<napi::Result<Vec<(Address, U256)>>>()?;

            let timestamp: Option<U256> = timestamp.map_or(Ok(None), |timestamp| {
                BigInt::try_cast(timestamp).map(Option::Some)
            })?;

            builder
                .finalize(&mut *self.state.write().await, rewards, timestamp)
                .map_or_else(
                    |e| Err(napi::Error::new(Status::GenericFailure, e.to_string())),
                    |block| Ok(Block::from(Arc::new(block))),
                )
        } else {
            Err(napi::Error::new(
                Status::InvalidArg,
                "The BlockBuilder object has been moved in Rust".to_owned(),
            ))
        }
    }

    /// This call consumes the [`BlockBuilder`] object in Rust. Afterwards, you can no longer call
    /// methods on the JS object.
    #[napi]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn abort(&self) -> napi::Result<()> {
        let mut builder = self.builder.write().await;
        if let Some(builder) = builder.take() {
            builder
                .abort(&mut *self.state.write().await)
                .map_err(|e| napi::Error::new(Status::GenericFailure, e.to_string()))
        } else {
            Err(napi::Error::new(
                Status::InvalidArg,
                "The BlockBuilder object has been moved in Rust".to_owned(),
            ))
        }
    }
}