mod body;
pub mod connect;
mod session;
pub mod types;

use futures::StreamExt;
use std::fmt::Debug;

use async_imap::{
    extensions::idle::{self, IdleResponse},
    types::{Fetch, Mailbox},
    Session,
};
use chrono::{Duration, Utc};
use imap_proto::{MailboxDatum, Response};
use itertools::Itertools;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::imap::body::parse_body;
use types::Message;

pub use session::ImapSession;

#[derive(Debug, thiserror::Error)]
pub enum ImapListenError {
    #[error("{0}")]
    Imap(#[from] async_imap::error::Error),

    #[error("Imap server idle send 'EXIT'")]
    Exit,
}

pub async fn imap_lookback<T>(
    session: &mut Session<T>,
    duration: Duration,
) -> Result<Vec<Message>, ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let from_date = Utc::now() - duration;
    let formatted = from_date.format("%d-%b-%Y");
    let uids = session.search(format!("UNSEEN SINCE {formatted}")).await?;

    let seq: String = uids.into_iter().map(|v| v.to_string()).join(",");

    fetch_seq(session, &seq)
        .await?
        .into_iter()
        .flat_map(|v| match v {
            MessageParseResult::Message(m) => Some(Ok(m)),
            MessageParseResult::ImapError(e) => Some(Err(ImapListenError::Imap(e))),
            MessageParseResult::BodyNotFound(_) => None,
        })
        .collect()
}

#[tracing::instrument(skip(session))]
pub async fn imap_idle<T>(
    mailbox: &mut Mailbox,
    mut session: Session<T>,
) -> Result<(Session<T>, Vec<Message>), ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let mut idle = session.idle();
    idle.init().await?;

    let (idle_wait, _) = idle.wait();

    let response = idle_wait.await?;
    let (idle, result) = handle_idle_response(idle, response).await?;

    session = idle.done().await?;
    let messages = handle_idle_event(&mut session, mailbox, result).await?;

    Ok((session, messages))
}

pub async fn handle_idle_response<T>(
    idle: idle::Handle<T>,
    response: IdleResponse,
) -> Result<(idle::Handle<T>, IdleEvent), ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match response {
        IdleResponse::ManualInterrupt => {
            idle.done().await?;
            Err(ImapListenError::Exit)
        }
        IdleResponse::Timeout => {
            idle.done().await?;
            Err(ImapListenError::Exit)
        }
        IdleResponse::NewData(data) => {
            tracing::debug!("New data: {:?}", data);
            if let Some(event) = parse_response(data.parsed()) {
                Ok((idle, event))
            } else {
                Err(ImapListenError::Exit)
            }
        }
    }
}

pub async fn handle_idle_event<T>(
    session: &mut Session<T>,
    mailbox: &mut Mailbox,
    event: IdleEvent,
) -> Result<Vec<Message>, ImapListenError>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    match event {
        IdleEvent::Exit => {
            return Err(ImapListenError::Exit);
        }
        IdleEvent::SizeDecrease(change) => {
            mailbox.exists -= change;
            return Ok(vec![]);
        }
        IdleEvent::Exists(new_size) => {
            let seq = format!("{}:{}", mailbox.exists + 1, new_size);
            mailbox.exists = new_size;
            let messages = fetch_seq(session, &seq)
                .await?
                .into_iter()
                .filter_map(|v| match v {
                    MessageParseResult::Message(m) => Some(m),
                    _ => None,
                })
                .collect();

            return Ok(messages);
        }
    };
}

fn parse_response(response: &Response) -> Option<IdleEvent> {
    match response {
        Response::Capabilities(_) => None,
        Response::Continue { .. } => None,
        Response::Done { .. } => None,
        Response::Data { .. } => None,
        Response::Expunge(_) => Some(IdleEvent::SizeDecrease(1)),
        Response::Vanished { .. } => None,
        Response::Fetch(..) => None,
        Response::MailboxData(mailbox) => match mailbox {
            MailboxDatum::Exists(uid) => Some(IdleEvent::Exists(*uid)),
            MailboxDatum::Flags(_) => None,
            MailboxDatum::List { .. } => None,
            MailboxDatum::Search(_) => None,
            MailboxDatum::Sort(_) => None,
            MailboxDatum::Status { .. } => None,
            MailboxDatum::Recent(_) => None,
            MailboxDatum::MetadataSolicited { .. } => None,
            MailboxDatum::MetadataUnsolicited { .. } => None,
            MailboxDatum::GmailLabels(_) => None,
            MailboxDatum::GmailMsgId(_) => None,
            _ => None,
        },
        Response::Quota(_) => None,
        Response::QuotaRoot(_) => None,
        Response::Id(_) => None,
        Response::Acl(_) => None,
        Response::ListRights(_) => None,
        Response::MyRights(_) => None,
        _ => None,
    }
}

pub enum IdleEvent {
    Exit,
    Exists(u32),
    SizeDecrease(u32),
}

#[derive(Debug)]
pub enum MessageParseResult {
    Message(Message),
    ImapError(async_imap::error::Error),
    BodyNotFound(Fetch),
}

#[tracing::instrument(skip(session))]
pub async fn fetch_seq<T>(
    session: &mut Session<T>,
    seq: &str,
) -> async_imap::error::Result<Vec<MessageParseResult>>
where
    T: AsyncRead + AsyncWrite + Debug + Send + Unpin,
{
    let mut messages = session
        .fetch(seq, "(FLAGS INTERNALDATE RFC822 BODY[] UID)")
        .await?;

    let mut parsed = vec![];
    while let Some(message) = messages.next().await {
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                parsed.push(MessageParseResult::ImapError(e));
                continue;
            }
        };

        let Some(body) = message.body() else {
            parsed.push(MessageParseResult::BodyNotFound(message));
            continue;
        };

        let parsed_message = parse_body(body)
            .map(MessageParseResult::Message)
            .unwrap_or_else(|| MessageParseResult::BodyNotFound(message));

        parsed.push(parsed_message);
    }

    Ok(parsed)
}
