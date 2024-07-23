use chrono::{DateTime, Utc};
use itertools::Itertools;
use mail_parser::{ContentType, MessageParser, MimeHeaders};

use crate::types::{
    Addr, Address, Attachment, ContentDisposition, Envelope, Group, Header, Message,
};

pub fn parse_body(raw_body: &[u8]) -> Option<Message> {
    let message = MessageParser::default().parse(raw_body)?;

    let envelope = Envelope {
        message_id: message.message_id().map(|v| v.to_string()),
        from: parser_address_into_internal(message.from()),
        to: parser_address_into_internal(message.to()),
        cc: parser_address_into_internal(message.cc()),
        bcc: parser_address_into_internal(message.bcc()),
        sender: parser_address_into_internal(message.sender()),
        date: message.date().map(parser_datetime_into_chrono).flatten(),
    };

    let resent = Envelope {
        message_id: message.resent_message_id().as_text().map(|v| v.to_string()),
        from: parser_address_into_internal(message.resent_from()),
        to: parser_address_into_internal(message.resent_to()),
        cc: parser_address_into_internal(message.resent_cc()),
        bcc: parser_address_into_internal(message.resent_bcc()),
        sender: parser_address_into_internal(message.resent_sender()),
        date: parser_header_into_date_time(message.resent_date()),
    };

    let body_text = message.text_bodies().map(|v| v.to_string()).collect();
    let body_html = message.html_bodies().map(|v| v.to_string()).collect();

    let attachments = message
        .attachments()
        .map(parse_attachment_into_internal)
        .collect();

    let headers = message
        .headers_raw()
        .map(|(n, v)| Header {
            name: n.to_string(),
            value: v.to_string(),
        })
        .collect();

    Some(Message {
        envelope,
        resent: resent.into_option(),
        reply_to: parser_address_into_internal(message.reply_to()),
        in_reply_to: message.in_reply_to().as_text().map(ToString::to_string),
        subject: message.subject().map(ToString::to_string),
        body_text,
        body_html,
        attachments,
        headers,
    })
}

fn parser_address_into_internal(address: Option<&mail_parser::Address<'_>>) -> Option<Address> {
    let Some(address) = address else { return None };

    match address {
        mail_parser::Address::List(v) => {
            let addresses = v
                .iter()
                .filter_map(|v| parser_addr_into_internal(v))
                .collect::<Vec<_>>();
            Some(Address::List(addresses))
        }
        mail_parser::Address::Group(groups) => {
            let internal_groups = groups
                .iter()
                .map(parser_group_into_internal)
                .collect::<Vec<_>>();
            Some(Address::Group(internal_groups))
        }
    }
}

fn parser_group_into_internal(group: &mail_parser::Group<'_>) -> Group {
    let addresses = group
        .addresses
        .iter()
        .filter_map(|v| parser_addr_into_internal(v))
        .collect::<Vec<_>>();
    Group {
        name: group.name.as_ref().map(|v| v.to_string()),
        addresses,
    }
}

fn parser_addr_into_internal(addr: &mail_parser::Addr<'_>) -> Option<Addr> {
    let Some(address) = addr.address() else {
        return None;
    };
    let parsed = Addr {
        name: addr.name().map(|v| v.to_string()),
        email: address.to_string(),
    };
    Some(parsed)
}

fn parser_header_into_date_time(value: &mail_parser::HeaderValue) -> Option<DateTime<Utc>> {
    value
        .as_datetime()
        .map(parser_datetime_into_chrono)
        .flatten()
}

fn parser_datetime_into_chrono(dt: &mail_parser::DateTime) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(dt.to_timestamp(), 0)
}

fn parse_attachment_into_internal(part: &mail_parser::MessagePart<'_>) -> Attachment {
    let content_disposition = part
        .content_disposition()
        .map(content_type_into_disposition)
        .flatten();

    Attachment {
        filename: part.attachment_name().map(ToString::to_string),
        content_id: part.content_id().map(ToString::to_string),
        content: part.contents().to_vec(),
        content_type: content_type_into_mime_type(part.content_type()),
        content_disposition,
        content_description: part.content_description().map(ToString::to_string),
        content_location: part.content_location().map(ToString::to_string),
    }
}

fn content_type_into_disposition(content_type: &ContentType<'_>) -> Option<ContentDisposition> {
    if content_type.is_attachment() {
        Some(ContentDisposition::Attachment)
    } else if content_type.is_inline() {
        Some(ContentDisposition::Inline)
    } else {
        None
    }
}

fn content_type_into_mime_type(content_type: Option<&ContentType<'_>>) -> Option<String> {
    let ctype = content_type.map(|ct| ct.ctype());
    let csubtype = content_type.map(|ct| ct.subtype()).flatten();
    let mime_type = [ctype, csubtype]
        .into_iter()
        .map(|v| v.unwrap_or_default())
        .join("/");

    if mime_type.is_empty() {
        None
    } else {
        Some(mime_type)
    }
}
