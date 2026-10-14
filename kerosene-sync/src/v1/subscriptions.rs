use futures::Stream;

use crate::connection::Input;
use crate::prelude::*;

/// manager for all the subscriptions for a connection
pub struct Subscriptions {
    // multiplex all events into a single stream
    // event_tx: mpsc::UnboundedSender<Result<MessageSync>>,
    // event_rx: mpsc::UnboundedReceiver<Result<MessageSync>>,

    // documents: HashMap<(ChannelId, DocumentBranchId), JoinHandle<()>>,
    // scripts: HashMap<(ChannelId, RedexId), JoinHandle<()>>,
    // member_lists: HashMap<String, (JoinHandle<()>, Vec<(u64, u64)>)>, // store ranges to detect when ranges are updated

    // documents: HashMap<(ChannelId, DocumentBranchId), Syncer<Sub = ()>>,
}

pub trait Syncer {
    type Sub;

    // fn set_subscription(&mut self, subscription: Self::Sub);
    // fn poll(&mut self) -> impl Future<Output = Result<MessageSync>> + Send;
}

// pub type DocumentSyncer = Syncer<Sub = Vec<lamprey::v1::types::sync::SyncSubscribeDocument>>;

// pub trait ServiceFoo {
//     fn create(&self, connection_id: ConnectionId, user_id: Option<UserId>) -> Syncer;
// }

// pub fn create_syncer(&self, conn_id: uuid::Uuid) -> syncer::MemberListSyncer {
// pub trait Syncer {
//     pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
//     pub async fn poll(&mut self) -> Result<MessageSync> {

//     // member list
//     pub async fn set_query( &mut self, target: MemberListTarget, ranges: &[(u64, u64)], ) -> Result<()> {
//     pub async fn clear_query(&mut self) {
//     pub async fn subscribe(&mut self, key1: MemberListKey1, ranges: Vec<(u64, u64)>) -> Result<()> {
//     pub async fn unsubscribe(&mut self, key1: MemberListKey1) -> Result<()> {

// // document
//     pub async fn set_context_id( &self, context_id: EditContextId, state_vector: Option<DocumentStateVector>, ) -> Result<()> {
//     pub fn is_subscribed(&self, context_id: &EditContextId) -> bool {
//     pub async fn handle_disconnect(&self, user_id: UserId) -> Result<()> {

//    // script
//     pub async fn set_context_id(&self, channel_id: ChannelId, script_id: RedexId) -> Result<()> {
//     pub fn is_subscribed(&self, channel_id: &ChannelId, script_id: &RedexId) -> bool {
// }

// unsure if i can impl these if i make ConnectionSubscriptions2 minimal
impl Subscriptions {
    pub async fn disconnect(&mut self) {
        todo!()
    }

    pub async fn handle_input(&mut self, input: Input) -> Result<()> {
        match input {
            Input::Timeout(_) => todo!(),
            Input::Recv(_) => todo!(),
            Input::Close(_) => todo!(),
        }
    }

    // pub async fn poll(&mut self) -> Result<MessageSync> {
    //     todo!()
    // }

    // pub fn stream(&self) -> impl Stream<Item = ()> {
    //     todo!()
    // }
}
