#![allow(clippy::unwrap_used)]

use asterisk_rs_ami::codec::RawAmiMessage;
use asterisk_rs_ami::event::AmiEvent;
use std::collections::HashMap;

/// build a raw message from header pairs
fn raw(headers: &[(&str, &str)]) -> RawAmiMessage {
    RawAmiMessage {
        headers: headers.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect(),
        output: vec![],
        channel_variables: HashMap::new(),
    }
}

// ── channel lifecycle ──

#[test]
fn parse_new_channel() {
    let msg = raw(&[("Event", "Newchannel"), ("Channel", "PJSIP/100-0001"), ("ChannelState", "0"), ("ChannelStateDesc", "Down"), ("CallerIDNum", "100"), ("CallerIDName", "Alice"), ("Uniqueid", "1234.1"), ("Linkedid", "1234.1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::NewChannel { channel, channel_state, channel_state_desc, caller_id_num, unique_id, linked_id, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
            assert_eq!(channel_state, "0");
            assert_eq!(channel_state_desc, "Down");
            assert_eq!(caller_id_num, "100");
            assert_eq!(unique_id, "1234.1");
            assert_eq!(linked_id, "1234.1");
        }
        other => panic!("expected NewChannel, got {other:?}"),
    }
}

#[test]
fn parse_hangup() {
    let msg = raw(&[("Event", "Hangup"), ("Channel", "SIP/100-0001"), ("Uniqueid", "1234.5"), ("Cause", "16"), ("Cause-txt", "Normal Clearing")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Hangup { channel, cause, cause_txt, .. } => {
            assert_eq!(channel, "SIP/100-0001");
            assert_eq!(cause, 16);
            assert_eq!(cause_txt, "Normal Clearing");
        }
        other => panic!("expected Hangup, got {other:?}"),
    }
}

#[test]
fn parse_newstate() {
    let msg = raw(&[("Event", "Newstate"), ("Channel", "PJSIP/100-0001"), ("ChannelState", "6"), ("ChannelStateDesc", "Up"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Newstate { channel, channel_state, channel_state_desc, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
            assert_eq!(channel_state, "6");
            assert_eq!(channel_state_desc, "Up");
        }
        other => panic!("expected Newstate, got {other:?}"),
    }
}

#[test]
fn parse_dial_begin() {
    let msg = raw(&[("Event", "DialBegin"), ("Channel", "PJSIP/100-0001"), ("DestChannel", "PJSIP/200-0002"), ("DialString", "200"), ("Uniqueid", "u1"), ("DestUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DialBegin { channel, destination, dial_string, dest_unique_id, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
            assert_eq!(destination, "PJSIP/200-0002");
            assert_eq!(dial_string, "200");
            assert_eq!(dest_unique_id, "u2");
        }
        other => panic!("expected DialBegin, got {other:?}"),
    }
}

#[test]
fn parse_dial_end() {
    let msg = raw(&[("Event", "DialEnd"), ("Channel", "PJSIP/100-0001"), ("DestChannel", "PJSIP/200-0002"), ("DialStatus", "ANSWER"), ("Uniqueid", "u1"), ("DestUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DialEnd { dial_status, dest_unique_id, .. } => {
            assert_eq!(dial_status, "ANSWER");
            assert_eq!(dest_unique_id, "u2");
        }
        other => panic!("expected DialEnd, got {other:?}"),
    }
}

#[test]
fn parse_dtmf_begin() {
    let msg = raw(&[("Event", "DTMFBegin"), ("Channel", "PJSIP/100-0001"), ("Digit", "5"), ("Direction", "Received"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DtmfBegin { digit, direction, .. } => {
            assert_eq!(digit, "5");
            assert_eq!(direction, "Received");
        }
        other => panic!("expected DtmfBegin, got {other:?}"),
    }
}

#[test]
fn parse_dtmf_end() {
    let msg = raw(&[("Event", "DTMFEnd"), ("Channel", "PJSIP/100-0001"), ("Digit", "5"), ("DurationMs", "120"), ("Direction", "Received"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DtmfEnd { digit, duration_ms, direction, .. } => {
            assert_eq!(digit, "5");
            assert_eq!(duration_ms, 120);
            assert_eq!(direction, "Received");
        }
        other => panic!("expected DtmfEnd, got {other:?}"),
    }
}

#[test]
fn parse_fully_booted() {
    let msg = raw(&[("Event", "FullyBooted"), ("Status", "Fully Booted")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FullyBooted { status, .. } => {
            assert_eq!(status, "Fully Booted");
        }
        other => panic!("expected FullyBooted, got {other:?}"),
    }
}

#[test]
fn parse_peer_status() {
    let msg = raw(&[("Event", "PeerStatus"), ("ChannelType", "PJSIP"), ("Peer", "PJSIP/100"), ("PeerStatus", "Reachable")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::PeerStatus { channel_type, peer, peer_status, .. } => {
            assert_eq!(channel_type, "PJSIP");
            assert_eq!(peer, "PJSIP/100");
            assert_eq!(peer_status, "Reachable");
        }
        other => panic!("expected PeerStatus, got {other:?}"),
    }
}

// ── bridge events ──

#[test]
fn parse_bridge_create() {
    let msg = raw(&[("Event", "BridgeCreate"), ("BridgeUniqueid", "br-1"), ("BridgeType", "basic")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeCreate { bridge_unique_id, bridge_type, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
            assert_eq!(bridge_type, "basic");
        }
        other => panic!("expected BridgeCreate, got {other:?}"),
    }
}

#[test]
fn parse_bridge_destroy() {
    let msg = raw(&[("Event", "BridgeDestroy"), ("BridgeUniqueid", "br-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeDestroy { bridge_unique_id, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
        }
        other => panic!("expected BridgeDestroy, got {other:?}"),
    }
}

#[test]
fn parse_bridge_enter() {
    let msg = raw(&[("Event", "BridgeEnter"), ("BridgeUniqueid", "br-1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeEnter { bridge_unique_id, channel, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected BridgeEnter, got {other:?}"),
    }
}

#[test]
fn parse_bridge_leave() {
    let msg = raw(&[("Event", "BridgeLeave"), ("BridgeUniqueid", "br-1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeLeave { bridge_unique_id, channel, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected BridgeLeave, got {other:?}"),
    }
}

#[test]
fn parse_bridge_merge() {
    let msg = raw(&[("Event", "BridgeMerge"), ("BridgeUniqueid", "br-1"), ("BridgeType", "basic"), ("ToBridgeUniqueid", "br-2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeMerge { bridge_unique_id, to_bridge_unique_id, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
            assert_eq!(to_bridge_unique_id, "br-2");
        }
        other => panic!("expected BridgeMerge, got {other:?}"),
    }
}

#[test]
fn parse_bridge_info_channel() {
    let msg = raw(&[("Event", "BridgeInfoChannel"), ("BridgeUniqueid", "br-1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeInfoChannel { bridge_unique_id, channel, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected BridgeInfoChannel, got {other:?}"),
    }
}

#[test]
fn parse_bridge_info_complete() {
    let msg = raw(&[("Event", "BridgeInfoComplete"), ("BridgeUniqueid", "br-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeInfoComplete { bridge_unique_id, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
        }
        other => panic!("expected BridgeInfoComplete, got {other:?}"),
    }
}

#[test]
fn parse_bridge_video_source_update() {
    let msg = raw(&[("Event", "BridgeVideoSourceUpdate"), ("BridgeUniqueid", "br-1"), ("BridgeVideoSourceUniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BridgeVideoSourceUpdate { bridge_unique_id, bridge_video_source_unique_id, .. } => {
            assert_eq!(bridge_unique_id, "br-1");
            assert_eq!(bridge_video_source_unique_id, "u1");
        }
        other => panic!("expected BridgeVideoSourceUpdate, got {other:?}"),
    }
}

// ── core call flow ──

#[test]
fn parse_var_set() {
    let msg = raw(&[("Event", "VarSet"), ("Channel", "PJSIP/100-0001"), ("Variable", "DIALSTATUS"), ("Value", "ANSWER"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::VarSet { variable, value, .. } => {
            assert_eq!(variable, "DIALSTATUS");
            assert_eq!(value, "ANSWER");
        }
        other => panic!("expected VarSet, got {other:?}"),
    }
}

#[test]
fn parse_hold() {
    let msg = raw(&[("Event", "Hold"), ("Channel", "PJSIP/200-0002"), ("Uniqueid", "u1"), ("MusicClass", "default")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Hold { channel, .. } => {
            assert_eq!(channel, "PJSIP/200-0002");
        }
        other => panic!("expected Hold, got {other:?}"),
    }
}

#[test]
fn parse_hold_without_music_class() {
    let msg = raw(&[("Event", "Hold"), ("Channel", "PJSIP/200-0002"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Hold { music_class, .. } => assert!(music_class.is_none()),
        other => panic!("expected Hold, got {other:?}"),
    }
}

#[test]
fn parse_unhold() {
    let msg = raw(&[("Event", "Unhold"), ("Channel", "PJSIP/200-0002"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Unhold { channel, unique_id, .. } => {
            assert_eq!(channel, "PJSIP/200-0002");
            assert_eq!(unique_id, "u1");
        }
        other => panic!("expected Unhold, got {other:?}"),
    }
}

#[test]
fn parse_hangup_request() {
    let msg = raw(&[("Event", "HangupRequest"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Cause", "16")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::HangupRequest { cause, .. } => {
            assert_eq!(cause, 16);
        }
        other => panic!("expected HangupRequest, got {other:?}"),
    }
}

#[test]
fn parse_soft_hangup_request() {
    let msg = raw(&[("Event", "SoftHangupRequest"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Cause", "32")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::SoftHangupRequest { cause, .. } => {
            assert_eq!(cause, 32);
        }
        other => panic!("expected SoftHangupRequest, got {other:?}"),
    }
}

#[test]
fn parse_new_exten() {
    let msg = raw(&[("Event", "NewExten"), ("Channel", "PJSIP/100-0001"), ("Context", "default"), ("Extension", "200"), ("Priority", "1"), ("Application", "Dial"), ("AppData", "PJSIP/200"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::NewExten { context, extension, priority, application, .. } => {
            assert_eq!(context, "default");
            assert_eq!(extension, "200");
            assert_eq!(priority, 1);
            assert_eq!(application, "Dial");
        }
        other => panic!("expected NewExten, got {other:?}"),
    }
}

#[test]
fn parse_new_callerid() {
    let msg = raw(&[("Event", "NewCallerid"), ("Channel", "PJSIP/100-0001"), ("CallerIDNum", "100"), ("CallerIDName", "Alice"), ("Uniqueid", "u1"), ("CID-CallingPres", "0 (Presentation Allowed)")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::NewCallerid { caller_id_num, caller_id_name, cid_calling_pres, .. } => {
            assert_eq!(caller_id_num, "100");
            assert_eq!(caller_id_name, "Alice");
            assert_eq!(cid_calling_pres, "0 (Presentation Allowed)");
        }
        other => panic!("expected NewCallerid, got {other:?}"),
    }
}

#[test]
fn parse_new_connected_line() {
    let msg = raw(&[("Event", "NewConnectedLine"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ConnectedLineNum", "200"), ("ConnectedLineName", "Bob")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::NewConnectedLine { connected_line_num, connected_line_name, .. } => {
            assert_eq!(connected_line_num, "200");
            assert_eq!(connected_line_name, "Bob");
        }
        other => panic!("expected NewConnectedLine, got {other:?}"),
    }
}

#[test]
fn parse_new_account_code() {
    let msg = raw(&[("Event", "NewAccountCode"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("AccountCode", "new-acct"), ("OldAccountCode", "old-acct")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::NewAccountCode { account_code, old_account_code, .. } => {
            assert_eq!(account_code, "new-acct");
            assert_eq!(old_account_code, "old-acct");
        }
        other => panic!("expected NewAccountCode, got {other:?}"),
    }
}

#[test]
fn parse_rename() {
    let msg = raw(&[("Event", "Rename"), ("Channel", "PJSIP/100-0001"), ("Newname", "PJSIP/100-0001<MASQ>"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Rename { channel, new_name, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
            assert_eq!(new_name, "PJSIP/100-0001<MASQ>");
        }
        other => panic!("expected Rename, got {other:?}"),
    }
}

#[test]
fn parse_originate_response() {
    let msg = raw(&[("Event", "OriginateResponse"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Response", "Success"), ("Reason", "4")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::OriginateResponse { response, reason, .. } => {
            assert_eq!(response, "Success");
            assert_eq!(reason, "4");
        }
        other => panic!("expected OriginateResponse, got {other:?}"),
    }
}

#[test]
fn parse_dial_state() {
    let msg = raw(&[("Event", "DialState"), ("Channel", "PJSIP/100-0001"), ("DestChannel", "PJSIP/200-0002"), ("DialStatus", "RINGING"), ("Uniqueid", "u1"), ("DestUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DialState { dial_status, dest_unique_id, .. } => {
            assert_eq!(dial_status, "RINGING");
            assert_eq!(dest_unique_id, "u2");
        }
        other => panic!("expected DialState, got {other:?}"),
    }
}

#[test]
fn parse_flash() {
    let msg = raw(&[("Event", "Flash"), ("Channel", "DAHDI/1-1"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Flash { channel, .. } => {
            assert_eq!(channel, "DAHDI/1-1");
        }
        other => panic!("expected Flash, got {other:?}"),
    }
}

#[test]
fn parse_wink() {
    let msg = raw(&[("Event", "Wink"), ("Channel", "DAHDI/1-1"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Wink { channel, .. } => {
            assert_eq!(channel, "DAHDI/1-1");
        }
        other => panic!("expected Wink, got {other:?}"),
    }
}

#[test]
fn parse_user_event() {
    let msg = raw(&[
        ("Event", "UserEvent"),
        ("Channel", "PJSIP/100-0001"),
        ("Uniqueid", "u1"),
        ("UserEvent", "MyCustomEvent"),
        ("CustomKey", "CustomVal"),
    ]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::UserEvent { channel, unique_id, user_event, .. } => {
            assert_eq!(channel.as_deref(), Some("PJSIP/100-0001"));
            assert_eq!(unique_id.as_deref(), Some("u1"));
            assert_eq!(user_event, "MyCustomEvent");
        }
        other => panic!("expected UserEvent, got {other:?}"),
    }
}

#[test]
fn parse_user_event_without_channel() {
    let msg = raw(&[("Event", "UserEvent"), ("UserEvent", "Ping")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::UserEvent { channel, unique_id, .. } => {
            assert!(channel.is_none());
            assert!(unique_id.is_none());
        }
        other => panic!("expected UserEvent, got {other:?}"),
    }
}

// ── transfer ──

#[test]
fn parse_attended_transfer() {
    let msg = raw(&[("Event", "AttendedTransfer"), ("Result", "Success"), ("TransfererChannel", "PJSIP/100-0001"), ("TransfererUniqueid", "u1"), ("TransfereeChannel", "PJSIP/200-0002"), ("TransfereeUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AttendedTransfer { result, transferer_channel, transferee_channel, .. } => {
            assert_eq!(result, "Success");
            assert_eq!(transferer_channel, "PJSIP/100-0001");
            assert_eq!(transferee_channel, "PJSIP/200-0002");
        }
        other => panic!("expected AttendedTransfer, got {other:?}"),
    }
}

#[test]
fn parse_blind_transfer() {
    let msg = raw(&[("Event", "BlindTransfer"), ("Result", "Success"), ("TransfererChannel", "PJSIP/100-0001"), ("TransfererUniqueid", "u1"), ("Extension", "300"), ("Context", "default")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::BlindTransfer { result, extension, context, .. } => {
            assert_eq!(result, "Success");
            assert_eq!(extension, "300");
            assert_eq!(context, "default");
        }
        other => panic!("expected BlindTransfer, got {other:?}"),
    }
}

// ── local channel ──

#[test]
fn parse_local_bridge() {
    let msg = raw(&[("Event", "LocalBridge"), ("Channel", "Local/100@default-0001"), ("Uniqueid", "u1"), ("Context", "default"), ("Exten", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::LocalBridge { context, exten, .. } => {
            assert_eq!(context, "default");
            assert_eq!(exten, "100");
        }
        other => panic!("expected LocalBridge, got {other:?}"),
    }
}

#[test]
fn parse_local_optimization_begin() {
    let msg = raw(&[("Event", "LocalOptimizationBegin"), ("Channel", "Local/100@default-0001"), ("Uniqueid", "u1"), ("SourceUniqueid", "s1"), ("DestUniqueid", "d1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::LocalOptimizationBegin { source_unique_id, dest_unique_id, .. } => {
            assert_eq!(source_unique_id, "s1");
            assert_eq!(dest_unique_id, "d1");
        }
        other => panic!("expected LocalOptimizationBegin, got {other:?}"),
    }
}

#[test]
fn parse_local_optimization_end() {
    let msg = raw(&[("Event", "LocalOptimizationEnd"), ("Channel", "Local/100@default-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::LocalOptimizationEnd { channel, .. } => {
            assert_eq!(channel, "Local/100@default-0001");
        }
        other => panic!("expected LocalOptimizationEnd, got {other:?}"),
    }
}

// ── cdr / cel ──

#[test]
fn parse_cdr() {
    let msg = raw(&[("Event", "Cdr"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Destination", "200"), ("Disposition", "ANSWERED"), ("Duration", "45"), ("BillableSeconds", "40"), ("AccountCode", "acct1"), ("Source", "100"), ("DestinationContext", "default")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Cdr { disposition, duration, billable_seconds, .. } => {
            assert_eq!(disposition, "ANSWERED");
            assert_eq!(duration, 45);
            assert_eq!(billable_seconds, 40);
        }
        other => panic!("expected Cdr, got {other:?}"),
    }
}

#[test]
fn parse_cel() {
    let msg = raw(&[("Event", "CEL"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("EventName", "CHAN_START"), ("AccountCode", "acct1"), ("ApplicationName", "Dial"), ("ApplicationData", "PJSIP/200")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Cel { event_name_cel, application_name, .. } => {
            assert_eq!(event_name_cel, "CHAN_START");
            assert_eq!(application_name, "Dial");
        }
        other => panic!("expected Cel, got {other:?}"),
    }
}

// ── queue events ──

#[test]
fn parse_queue_caller_abandon() {
    let msg = raw(&[("Event", "QueueCallerAbandon"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Queue", "support"), ("Position", "2"), ("OriginalPosition", "1"), ("HoldTime", "30")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueCallerAbandon { queue, position, original_position, hold_time, .. } => {
            assert_eq!(queue, "support");
            assert_eq!(position, 2);
            assert_eq!(original_position, 1);
            assert_eq!(hold_time, 30);
        }
        other => panic!("expected QueueCallerAbandon, got {other:?}"),
    }
}

#[test]
fn parse_queue_caller_join() {
    let msg = raw(&[("Event", "QueueCallerJoin"), ("Channel", "PJSIP/300-0003"), ("Uniqueid", "u1"), ("Queue", "support"), ("Position", "1"), ("Count", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueCallerJoin { queue, position, count, .. } => {
            assert_eq!(queue, "support");
            assert_eq!(position, 1);
            assert_eq!(count, 3);
        }
        other => panic!("expected QueueCallerJoin, got {other:?}"),
    }
}

#[test]
fn parse_queue_caller_leave() {
    let msg = raw(&[("Event", "QueueCallerLeave"), ("Channel", "PJSIP/300-0003"), ("Uniqueid", "u1"), ("Queue", "support"), ("Position", "2"), ("Count", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueCallerLeave { position, count, .. } => {
            assert_eq!(position, 2);
            assert_eq!(count, 1);
        }
        other => panic!("expected QueueCallerLeave, got {other:?}"),
    }
}

#[test]
fn parse_queue_member_added() {
    let msg = raw(&[("Event", "QueueMemberAdded"), ("Queue", "support"), ("MemberName", "Agent/100"), ("Interface", "PJSIP/100"), ("StateInterface", "PJSIP/100"), ("Membership", "dynamic"), ("Penalty", "5"), ("Paused", "0")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueMemberAdded { queue, member_name, penalty, paused, .. } => {
            assert_eq!(queue, "support");
            assert_eq!(member_name, "Agent/100");
            assert_eq!(penalty, 5);
            assert_eq!(paused, "0");
        }
        other => panic!("expected QueueMemberAdded, got {other:?}"),
    }
}

#[test]
fn parse_queue_member_removed() {
    let msg = raw(&[("Event", "QueueMemberRemoved"), ("Queue", "support"), ("MemberName", "Agent/100"), ("Interface", "PJSIP/100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueMemberRemoved { queue, member_name, .. } => {
            assert_eq!(queue, "support");
            assert_eq!(member_name, "Agent/100");
        }
        other => panic!("expected QueueMemberRemoved, got {other:?}"),
    }
}

#[test]
fn parse_queue_member_pause() {
    let msg = raw(&[("Event", "QueueMemberPause"), ("Queue", "support"), ("MemberName", "Agent/100"), ("Interface", "PJSIP/100"), ("Paused", "1"), ("Reason", "break")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueMemberPause { paused, reason, .. } => {
            assert_eq!(paused, "1");
            assert_eq!(reason, "break");
        }
        other => panic!("expected QueueMemberPause, got {other:?}"),
    }
}

#[test]
fn parse_queue_member_status() {
    let msg = raw(&[("Event", "QueueMemberStatus"), ("Queue", "support"), ("MemberName", "Agent/100"), ("Interface", "PJSIP/100"), ("Status", "1"), ("Paused", "0"), ("CallsTaken", "10")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueMemberStatus { status, calls_taken, .. } => {
            assert_eq!(status, 1);
            assert_eq!(calls_taken, 10);
        }
        other => panic!("expected QueueMemberStatus, got {other:?}"),
    }
}

#[test]
fn parse_queue_member_penalty() {
    let msg = raw(&[("Event", "QueueMemberPenalty"), ("Queue", "support"), ("MemberName", "Agent/100"), ("Interface", "PJSIP/100"), ("Penalty", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueMemberPenalty { penalty, .. } => {
            assert_eq!(penalty, 3);
        }
        other => panic!("expected QueueMemberPenalty, got {other:?}"),
    }
}

#[test]
fn parse_queue_member_ringinuse() {
    let msg = raw(&[("Event", "QueueMemberRinginuse"), ("Queue", "support"), ("MemberName", "Agent/100"), ("Interface", "PJSIP/100"), ("Ringinuse", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueMemberRinginuse { ringinuse, .. } => {
            assert_eq!(ringinuse, "1");
        }
        other => panic!("expected QueueMemberRinginuse, got {other:?}"),
    }
}

#[test]
fn parse_queue_params() {
    let msg = raw(&[("Event", "QueueParams"), ("Queue", "support"), ("Max", "10"), ("Strategy", "ringall"), ("Calls", "5"), ("Holdtime", "30"), ("Talktime", "120"), ("Completed", "50"), ("Abandoned", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueParams { queue, max, strategy, calls, holdtime, completed, abandoned, .. } => {
            assert_eq!(queue, "support");
            assert_eq!(max, 10);
            assert_eq!(strategy, "ringall");
            assert_eq!(calls, 5);
            assert_eq!(holdtime, 30);
            assert_eq!(completed, 50);
            assert_eq!(abandoned, 3);
        }
        other => panic!("expected QueueParams, got {other:?}"),
    }
}

#[test]
fn parse_queue_entry() {
    let msg = raw(&[("Event", "QueueEntry"), ("Queue", "support"), ("Position", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("CallerIDNum", "100"), ("CallerIDName", "Alice"), ("Wait", "15")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::QueueEntry { queue, position, caller_id_num, wait, .. } => {
            assert_eq!(queue, "support");
            assert_eq!(position, 1);
            assert_eq!(caller_id_num, "100");
            assert_eq!(wait, 15);
        }
        other => panic!("expected QueueEntry, got {other:?}"),
    }
}

// ── agent events ──

#[test]
fn parse_agent_called() {
    let msg = raw(&[("Event", "AgentCalled"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Queue", "support"), ("Agent", "100"), ("DestinationChannel", "PJSIP/200-0002")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentCalled { agent, destination_channel, .. } => {
            assert_eq!(agent, "100");
            assert_eq!(destination_channel, "PJSIP/200-0002");
        }
        other => panic!("expected AgentCalled, got {other:?}"),
    }
}

#[test]
fn parse_agent_connect() {
    let msg = raw(&[("Event", "AgentConnect"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Queue", "support"), ("Agent", "100"), ("HoldTime", "15"), ("BridgeUniqueid", "br-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentConnect { hold_time, bridge_unique_id, .. } => {
            assert_eq!(hold_time, 15);
            assert_eq!(bridge_unique_id, "br-1");
        }
        other => panic!("expected AgentConnect, got {other:?}"),
    }
}

#[test]
fn parse_agent_complete() {
    let msg = raw(&[("Event", "AgentComplete"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Queue", "support"), ("Agent", "100"), ("HoldTime", "10"), ("TalkTime", "60"), ("Reason", "caller")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentComplete { hold_time, talk_time, reason, .. } => {
            assert_eq!(hold_time, 10);
            assert_eq!(talk_time, 60);
            assert_eq!(reason, "caller");
        }
        other => panic!("expected AgentComplete, got {other:?}"),
    }
}

#[test]
fn parse_agent_dump() {
    let msg = raw(&[("Event", "AgentDump"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Queue", "support"), ("Agent", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentDump { agent, .. } => {
            assert_eq!(agent, "100");
        }
        other => panic!("expected AgentDump, got {other:?}"),
    }
}

#[test]
fn parse_agent_login() {
    let msg = raw(&[("Event", "AgentLogin"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Agent", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentLogin { agent, .. } => {
            assert_eq!(agent, "100");
        }
        other => panic!("expected AgentLogin, got {other:?}"),
    }
}

#[test]
fn parse_agent_logoff() {
    let msg = raw(&[("Event", "AgentLogoff"), ("Agent", "100"), ("Logintime", "3600")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentLogoff { agent, logintime, .. } => {
            assert_eq!(agent, "100");
            assert_eq!(logintime, 3600);
        }
        other => panic!("expected AgentLogoff, got {other:?}"),
    }
}

#[test]
fn parse_agent_ring_no_answer() {
    let msg = raw(&[("Event", "AgentRingNoAnswer"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Queue", "support"), ("Agent", "100"), ("RingTime", "15")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AgentRingNoAnswer { ring_time, .. } => {
            assert_eq!(ring_time, 15);
        }
        other => panic!("expected AgentRingNoAnswer, got {other:?}"),
    }
}

#[test]
fn parse_agents() {
    let msg = raw(&[
        ("Event", "Agents"),
        ("Agent", "100"),
        ("Name", "Agent Smith"),
        ("Status", "AGENT_IDLE"),
        ("Channel", "PJSIP/100-0001"),
    ]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Agents { agent, name, status, channel } => {
            assert_eq!(agent, "100");
            assert_eq!(name, "Agent Smith");
            assert_eq!(status, "AGENT_IDLE");
            assert_eq!(channel.as_deref(), Some("PJSIP/100-0001"));
        }
        other => panic!("expected Agents, got {other:?}"),
    }
}

#[test]
fn parse_agents_complete() {
    let msg = raw(&[("Event", "AgentsComplete")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(matches!(event, AmiEvent::AgentsComplete));
}

// ── conference events ──

#[test]
fn parse_confbridge_start() {
    let msg = raw(&[("Event", "ConfbridgeStart"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeStart { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeStart, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_end() {
    let msg = raw(&[("Event", "ConfbridgeEnd"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeEnd { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeEnd, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_join() {
    let msg = raw(&[("Event", "ConfbridgeJoin"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Admin", "Yes")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeJoin { conference, admin, .. } => {
            assert_eq!(conference, "conf-100");
            assert_eq!(admin, "Yes");
        }
        other => panic!("expected ConfbridgeJoin, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_leave() {
    let msg = raw(&[("Event", "ConfbridgeLeave"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeLeave { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeLeave, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_list() {
    let msg = raw(&[("Event", "ConfbridgeList"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Admin", "No"), ("Muted", "No")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeList { admin, muted, .. } => {
            assert_eq!(admin, "No");
            assert_eq!(muted, "No");
        }
        other => panic!("expected ConfbridgeList, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_mute() {
    let msg = raw(&[("Event", "ConfbridgeMute"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeMute { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeMute, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_unmute() {
    let msg = raw(&[("Event", "ConfbridgeUnmute"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeUnmute { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeUnmute, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_talking() {
    let msg = raw(&[("Event", "ConfbridgeTalking"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("TalkingStatus", "on")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeTalking { talking_status, .. } => {
            assert_eq!(talking_status, "on");
        }
        other => panic!("expected ConfbridgeTalking, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_record() {
    let msg = raw(&[("Event", "ConfbridgeRecord"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeRecord { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeRecord, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_stop_record() {
    let msg = raw(&[("Event", "ConfbridgeStopRecord"), ("BridgeUniqueid", "br-1"), ("Conference", "conf-100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeStopRecord { conference, .. } => {
            assert_eq!(conference, "conf-100");
        }
        other => panic!("expected ConfbridgeStopRecord, got {other:?}"),
    }
}

#[test]
fn parse_confbridge_list_rooms() {
    let msg = raw(&[("Event", "ConfbridgeListRooms"), ("Conference", "conf-100"), ("Parties", "5"), ("Marked", "1"), ("Locked", "No")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ConfbridgeListRooms { conference, parties, marked, locked, .. } => {
            assert_eq!(conference, "conf-100");
            assert_eq!(parties, 5);
            assert_eq!(marked, 1);
            assert_eq!(locked, "No");
        }
        other => panic!("expected ConfbridgeListRooms, got {other:?}"),
    }
}

// ── mixmonitor ──

#[test]
fn parse_mix_monitor_start() {
    let msg = raw(&[("Event", "MixMonitorStart"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MixMonitorStart { channel, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected MixMonitorStart, got {other:?}"),
    }
}

#[test]
fn parse_mix_monitor_stop() {
    let msg = raw(&[("Event", "MixMonitorStop"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MixMonitorStop { channel, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected MixMonitorStop, got {other:?}"),
    }
}

#[test]
fn parse_mix_monitor_mute() {
    let msg = raw(&[("Event", "MixMonitorMute"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Direction", "read"), ("State", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MixMonitorMute { direction, state, .. } => {
            assert_eq!(direction, "read");
            assert_eq!(state, "1");
        }
        other => panic!("expected MixMonitorMute, got {other:?}"),
    }
}

// ── music on hold ──

#[test]
fn parse_music_on_hold_start() {
    let msg = raw(&[("Event", "MusicOnHoldStart"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Class", "default")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MusicOnHoldStart { class, .. } => {
            assert_eq!(class, "default");
        }
        other => panic!("expected MusicOnHoldStart, got {other:?}"),
    }
}

#[test]
fn parse_music_on_hold_stop() {
    let msg = raw(&[("Event", "MusicOnHoldStop"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MusicOnHoldStop { channel, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected MusicOnHoldStop, got {other:?}"),
    }
}

// ── parking ──

#[test]
fn parse_parked_call() {
    let msg = raw(&[("Event", "ParkedCall"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ParkingLot", "default"), ("ParkingSpace", "701"), ("ParkerDialString", "PJSIP/100"), ("Timeout", "45")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ParkedCall { parking_lot, parking_space, timeout, .. } => {
            assert_eq!(parking_lot, "default");
            assert_eq!(parking_space, 701);
            assert_eq!(timeout, 45);
        }
        other => panic!("expected ParkedCall, got {other:?}"),
    }
}

#[test]
fn parse_parked_call_give_up() {
    let msg = raw(&[("Event", "ParkedCallGiveUp"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ParkingLot", "default"), ("ParkingSpace", "701")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ParkedCallGiveUp { parking_space, .. } => {
            assert_eq!(parking_space, 701);
        }
        other => panic!("expected ParkedCallGiveUp, got {other:?}"),
    }
}

#[test]
fn parse_parked_call_time_out() {
    let msg = raw(&[("Event", "ParkedCallTimeOut"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ParkingLot", "default"), ("ParkingSpace", "701")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ParkedCallTimeOut { parking_space, .. } => {
            assert_eq!(parking_space, 701);
        }
        other => panic!("expected ParkedCallTimeOut, got {other:?}"),
    }
}

#[test]
fn parse_parked_call_swap() {
    let msg = raw(&[("Event", "ParkedCallSwap"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ParkingLot", "default"), ("ParkingSpace", "701"), ("ParkerChannel", "PJSIP/200-0002")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ParkedCallSwap { parking_space, parker_channel, .. } => {
            assert_eq!(parking_space, 701);
            assert_eq!(parker_channel, "PJSIP/200-0002");
        }
        other => panic!("expected ParkedCallSwap, got {other:?}"),
    }
}

#[test]
fn parse_unparked_call() {
    let msg = raw(&[("Event", "UnParkedCall"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ParkingLot", "default"), ("ParkingSpace", "701"), ("RetrieverChannel", "PJSIP/300-0003")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::UnParkedCall { parking_space, retriever_channel, .. } => {
            assert_eq!(parking_space, 701);
            assert_eq!(retriever_channel, "PJSIP/300-0003");
        }
        other => panic!("expected UnParkedCall, got {other:?}"),
    }
}

// ── pickup / spy ──

#[test]
fn parse_pickup() {
    let msg = raw(&[("Event", "Pickup"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("TargetChannel", "PJSIP/200-0002"), ("TargetUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Pickup { target_channel, target_unique_id, .. } => {
            assert_eq!(target_channel, "PJSIP/200-0002");
            assert_eq!(target_unique_id, "u2");
        }
        other => panic!("expected Pickup, got {other:?}"),
    }
}

#[test]
fn parse_chan_spy_start() {
    let msg = raw(&[("Event", "ChanSpyStart"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("SpyeeChannel", "PJSIP/200-0002"), ("SpyeeUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ChanSpyStart { spy_channel, spy_unique_id, .. } => {
            assert_eq!(spy_channel, "PJSIP/200-0002");
            assert_eq!(spy_unique_id, "u2");
        }
        other => panic!("expected ChanSpyStart, got {other:?}"),
    }
}

#[test]
fn parse_chan_spy_stop() {
    let msg = raw(&[("Event", "ChanSpyStop"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("SpyeeChannel", "PJSIP/200-0002"), ("SpyeeUniqueid", "u2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ChanSpyStop { spy_channel, .. } => {
            assert_eq!(spy_channel, "PJSIP/200-0002");
        }
        other => panic!("expected ChanSpyStop, got {other:?}"),
    }
}

// ── channel talking ──

#[test]
fn parse_channel_talking_start() {
    let msg = raw(&[("Event", "ChannelTalkingStart"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ChannelTalkingStart { channel, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected ChannelTalkingStart, got {other:?}"),
    }
}

#[test]
fn parse_channel_talking_stop() {
    let msg = raw(&[("Event", "ChannelTalkingStop"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Duration", "120")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ChannelTalkingStop { duration, .. } => {
            assert_eq!(duration, 120);
        }
        other => panic!("expected ChannelTalkingStop, got {other:?}"),
    }
}

// ── device / presence / extension state ──

#[test]
fn parse_device_state_change() {
    let msg = raw(&[("Event", "DeviceStateChange"), ("Device", "PJSIP/100"), ("State", "NOT_INUSE")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DeviceStateChange { device, state, .. } => {
            assert_eq!(device, "PJSIP/100");
            assert_eq!(state, "NOT_INUSE");
        }
        other => panic!("expected DeviceStateChange, got {other:?}"),
    }
}

#[test]
fn parse_extension_status() {
    let msg = raw(&[("Event", "ExtensionStatus"), ("Exten", "100"), ("Context", "default"), ("Hint", "PJSIP/100"), ("Status", "0"), ("StatusText", "Idle")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ExtensionStatus { exten, status, status_text, .. } => {
            assert_eq!(exten, "100");
            assert_eq!(status, 0);
            assert_eq!(status_text, "Idle");
        }
        other => panic!("expected ExtensionStatus, got {other:?}"),
    }
}

#[test]
fn parse_presence_state_change() {
    let msg = raw(&[("Event", "PresenceStateChange"), ("Presentity", "100@default"), ("Status", "available"), ("Subtype", ""), ("Message", "On the phone")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::PresenceStateChange { presentity, status, .. } => {
            assert_eq!(presentity, "100@default");
            assert_eq!(status, "available");
        }
        other => panic!("expected PresenceStateChange, got {other:?}"),
    }
}

#[test]
fn parse_presence_status() {
    let msg = raw(&[("Event", "PresenceStatus"), ("Presentity", "100@default"), ("Status", "away"), ("Subtype", "meeting"), ("Message", "In meeting")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::PresenceStatus { status, message, .. } => {
            assert_eq!(status, "away");
            assert_eq!(message, "In meeting");
        }
        other => panic!("expected PresenceStatus, got {other:?}"),
    }
}

// ── pjsip / registration ──

#[test]
fn parse_contact_status() {
    let msg = raw(&[("Event", "ContactStatus"), ("URI", "sip:100@192.168.1.10:5060"), ("ContactStatus", "Created"), ("AOR", "100"), ("EndpointName", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ContactStatus { uri, contact_status, .. } => {
            assert_eq!(uri, "sip:100@192.168.1.10:5060");
            assert_eq!(contact_status, "Created");
        }
        other => panic!("expected ContactStatus, got {other:?}"),
    }
}

#[test]
fn parse_registry() {
    let msg = raw(&[("Event", "Registry"), ("ChannelType", "PJSIP"), ("Domain", "sip.example.com"), ("Username", "100"), ("Status", "Registered"), ("Cause", "")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Registry { domain, status, .. } => {
            assert_eq!(domain, "sip.example.com");
            assert_eq!(status, "Registered");
        }
        other => panic!("expected Registry, got {other:?}"),
    }
}

// ── message / voicemail ──

#[test]
fn parse_message_waiting() {
    let msg = raw(&[("Event", "MessageWaiting"), ("Mailbox", "100@default"), ("Waiting", "1"), ("New", "3"), ("Old", "5")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MessageWaiting { mailbox, new_messages, old_messages, .. } => {
            assert_eq!(mailbox, "100@default");
            assert_eq!(new_messages, 3);
            assert_eq!(old_messages, 5);
        }
        other => panic!("expected MessageWaiting, got {other:?}"),
    }
}

#[test]
fn parse_voicemail_password_change() {
    let msg = raw(&[("Event", "VoicemailPasswordChange"), ("Context", "default"), ("Mailbox", "100"), ("NewPassword", "1234")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::VoicemailPasswordChange { mailbox, new_password, .. } => {
            assert_eq!(mailbox, "100");
            assert_eq!(new_password, "1234");
        }
        other => panic!("expected VoicemailPasswordChange, got {other:?}"),
    }
}

// ── rtcp ──

#[test]
fn parse_rtcp_received() {
    let msg = raw(&[("Event", "RTCPReceived"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("SSRC", "0x12345678"), ("PT", "200"), ("From", "192.168.1.10:10000")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::RTCPReceived { ssrc, from, .. } => {
            assert_eq!(ssrc, "0x12345678");
            assert_eq!(from, "192.168.1.10:10000");
        }
        other => panic!("expected RTCPReceived, got {other:?}"),
    }
}

#[test]
fn parse_rtcp_sent() {
    let msg = raw(&[("Event", "RTCPSent"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("SSRC", "0xAABBCCDD"), ("PT", "200"), ("To", "192.168.1.10:10000")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::RTCPSent { ssrc, to, .. } => {
            assert_eq!(ssrc, "0xAABBCCDD");
            assert_eq!(to, "192.168.1.10:10000");
        }
        other => panic!("expected RTCPSent, got {other:?}"),
    }
}

// ── security events ──

#[test]
fn parse_failed_acl() {
    let msg = raw(&[("Event", "FailedACL"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FailedACL { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected FailedACL, got {other:?}"),
    }
}

#[test]
fn parse_invalid_account_id() {
    let msg = raw(&[("Event", "InvalidAccountID"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::InvalidAccountID { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected InvalidAccountID, got {other:?}"),
    }
}

#[test]
fn parse_invalid_password() {
    let msg = raw(&[("Event", "InvalidPassword"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::InvalidPassword { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected InvalidPassword, got {other:?}"),
    }
}

#[test]
fn parse_challenge_response_failed() {
    let msg = raw(&[("Event", "ChallengeResponseFailed"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ChallengeResponseFailed { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected ChallengeResponseFailed, got {other:?}"),
    }
}

#[test]
fn parse_challenge_sent() {
    let msg = raw(&[("Event", "ChallengeSent"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ChallengeSent { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected ChallengeSent, got {other:?}"),
    }
}

#[test]
fn parse_successful_auth() {
    let msg = raw(&[("Event", "SuccessfulAuth"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::SuccessfulAuth { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected SuccessfulAuth, got {other:?}"),
    }
}

#[test]
fn parse_session_limit() {
    let msg = raw(&[("Event", "SessionLimit"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::SessionLimit { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected SessionLimit, got {other:?}"),
    }
}

#[test]
fn parse_unexpected_address() {
    let msg = raw(&[("Event", "UnexpectedAddress"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::UnexpectedAddress { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected UnexpectedAddress, got {other:?}"),
    }
}

#[test]
fn parse_request_bad_format() {
    let msg = raw(&[("Event", "RequestBadFormat"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::RequestBadFormat { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected RequestBadFormat, got {other:?}"),
    }
}

#[test]
fn parse_request_not_allowed() {
    let msg = raw(&[("Event", "RequestNotAllowed"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::RequestNotAllowed { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected RequestNotAllowed, got {other:?}"),
    }
}

#[test]
fn parse_request_not_supported() {
    let msg = raw(&[("Event", "RequestNotSupported"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::RequestNotSupported { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected RequestNotSupported, got {other:?}"),
    }
}

#[test]
fn parse_invalid_transport() {
    let msg = raw(&[("Event", "InvalidTransport"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::InvalidTransport { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected InvalidTransport, got {other:?}"),
    }
}

#[test]
fn parse_auth_method_not_allowed() {
    let msg = raw(&[("Event", "AuthMethodNotAllowed"), ("Severity", "Error"), ("Service", "AMI"), ("AccountID", "admin"), ("RemoteAddress", "IPV4/TCP/192.168.1.100/5038")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AuthMethodNotAllowed { severity, service, account_id, remote_address, .. } => {
            assert_eq!(severity, "Error");
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
            assert_eq!(remote_address, "IPV4/TCP/192.168.1.100/5038");
        }
        other => panic!("expected AuthMethodNotAllowed, got {other:?}"),
    }
}

// ── system events ──

#[test]
fn parse_shutdown() {
    let msg = raw(&[("Event", "Shutdown"), ("Shutdown", "Cleanly"), ("Restart", "True")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Shutdown { shutdown_status, restart, .. } => {
            assert_eq!(shutdown_status, "Cleanly");
            assert_eq!(restart, "True");
        }
        other => panic!("expected Shutdown, got {other:?}"),
    }
}

#[test]
fn parse_reload() {
    let msg = raw(&[("Event", "Reload"), ("Module", "cdr_csv.so"), ("Status", "0")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Reload { module, status, .. } => {
            assert_eq!(module, "cdr_csv.so");
            assert_eq!(status, "0");
        }
        other => panic!("expected Reload, got {other:?}"),
    }
}

#[test]
fn parse_load() {
    let msg = raw(&[("Event", "Load"), ("Module", "res_pjsip.so"), ("Status", "0")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Load { module, .. } => {
            assert_eq!(module, "res_pjsip.so");
        }
        other => panic!("expected Load, got {other:?}"),
    }
}

#[test]
fn parse_unload() {
    let msg = raw(&[("Event", "Unload"), ("Module", "res_pjsip.so"), ("Status", "0")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Unload { module, .. } => {
            assert_eq!(module, "res_pjsip.so");
        }
        other => panic!("expected Unload, got {other:?}"),
    }
}

#[test]
fn parse_log_channel() {
    let msg = raw(&[("Event", "LogChannel"), ("Channel", "console"), ("Enabled", "Yes")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::LogChannel { channel_log, enabled, .. } => {
            assert_eq!(channel_log, "console");
            assert_eq!(enabled, "Yes");
        }
        other => panic!("expected LogChannel, got {other:?}"),
    }
}

#[test]
fn parse_load_average_limit() {
    let msg = raw(&[("Event", "LoadAverageLimit")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(matches!(event, AmiEvent::LoadAverageLimit));
}

#[test]
fn parse_memory_limit() {
    let msg = raw(&[("Event", "MemoryLimit")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(matches!(event, AmiEvent::MemoryLimit));
}

// ── async agi ──

#[test]
fn parse_async_agi_start() {
    let msg = raw(&[("Event", "AsyncAGIStart"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Env", "agi_request%3A%20async")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AsyncAGIStart { env, .. } => {
            assert_eq!(env, "agi_request%3A%20async");
        }
        other => panic!("expected AsyncAGIStart, got {other:?}"),
    }
}

#[test]
fn parse_async_agi_exec() {
    let msg = raw(&[("Event", "AsyncAGIExec"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("CommandID", "cmd-1"), ("Result", "200 result=1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AsyncAGIExec { command_id, result, .. } => {
            assert_eq!(command_id, "cmd-1");
            assert_eq!(result, "200 result=1");
        }
        other => panic!("expected AsyncAGIExec, got {other:?}"),
    }
}

#[test]
fn parse_async_agi_end() {
    let msg = raw(&[("Event", "AsyncAGIEnd"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AsyncAGIEnd { channel, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected AsyncAGIEnd, got {other:?}"),
    }
}

#[test]
fn parse_agi_exec_start() {
    let msg = raw(&[("Event", "AGIExecStart"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Command", "ANSWER"), ("CommandId", "cmd-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AGIExecStart { command, command_id, .. } => {
            assert_eq!(command, "ANSWER");
            assert_eq!(command_id, "cmd-1");
        }
        other => panic!("expected AGIExecStart, got {other:?}"),
    }
}

#[test]
fn parse_agi_exec_end() {
    let msg = raw(&[("Event", "AGIExecEnd"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Command", "ANSWER"), ("CommandId", "cmd-1"), ("ResultCode", "200"), ("Result", "result=0")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AGIExecEnd { result_code, result, .. } => {
            assert_eq!(result_code, "200");
            assert_eq!(result, "result=0");
        }
        other => panic!("expected AGIExecEnd, got {other:?}"),
    }
}

// ── hangup handlers ──

#[test]
fn parse_hangup_handler_push() {
    let msg = raw(&[("Event", "HangupHandlerPush"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Handler", "handler1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::HangupHandlerPush { handler, .. } => {
            assert_eq!(handler, "handler1");
        }
        other => panic!("expected HangupHandlerPush, got {other:?}"),
    }
}

#[test]
fn parse_hangup_handler_pop() {
    let msg = raw(&[("Event", "HangupHandlerPop"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Handler", "handler1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::HangupHandlerPop { handler, .. } => {
            assert_eq!(handler, "handler1");
        }
        other => panic!("expected HangupHandlerPop, got {other:?}"),
    }
}

#[test]
fn parse_hangup_handler_run() {
    let msg = raw(&[("Event", "HangupHandlerRun"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Handler", "handler1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::HangupHandlerRun { handler, .. } => {
            assert_eq!(handler, "handler1");
        }
        other => panic!("expected HangupHandlerRun, got {other:?}"),
    }
}

// ── core show / status ──

#[test]
fn parse_status() {
    let msg = raw(&[("Event", "Status"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ChannelState", "6"), ("CallerIDNum", "100"), ("CallerIDName", "Alice"), ("AccountCode", "acct1"), ("Context", "default"), ("Exten", "200"), ("Priority", "1"), ("Seconds", "30"), ("BridgeID", "br-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Status { channel_state, priority, seconds, bridge_id, .. } => {
            assert_eq!(channel_state, "6");
            assert_eq!(priority, 1);
            assert_eq!(seconds, 30);
            assert_eq!(bridge_id, "br-1");
        }
        other => panic!("expected Status, got {other:?}"),
    }
}

#[test]
fn parse_status_complete() {
    let msg = raw(&[("Event", "StatusComplete"), ("Items", "5")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::StatusComplete { items, .. } => {
            assert_eq!(items, 5);
        }
        other => panic!("expected StatusComplete, got {other:?}"),
    }
}

#[test]
fn parse_core_show_channel() {
    let msg = raw(&[("Event", "CoreShowChannel"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ChannelState", "6"), ("CallerIDNum", "100"), ("CallerIDName", "Alice"), ("Application", "Dial"), ("ApplicationData", "PJSIP/200"), ("Duration", "00:01:30"), ("BridgeID", "br-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::CoreShowChannel { application, duration, .. } => {
            assert_eq!(application, "Dial");
            assert_eq!(duration, "00:01:30");
        }
        other => panic!("expected CoreShowChannel, got {other:?}"),
    }
}

#[test]
fn parse_core_show_channels_complete() {
    let msg = raw(&[("Event", "CoreShowChannelsComplete"), ("ListItems", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::CoreShowChannelsComplete { listed_channels, .. } => {
            assert_eq!(listed_channels, 3);
        }
        other => panic!("expected CoreShowChannelsComplete, got {other:?}"),
    }
}

#[test]
fn parse_core_show_channel_map_complete() {
    let msg = raw(&[("Event", "CoreShowChannelMapComplete")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(matches!(event, AmiEvent::CoreShowChannelMapComplete));
}

// ── dahdi ──

#[test]
fn parse_dahdi_channel() {
    let msg = raw(&[
        ("Event", "DAHDIChannel"),
        ("DAHDIChannel", "1"),
        ("Channel", "DAHDI/1-1"),
        ("Uniqueid", "u1"),
    ]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DAHDIChannel { dahdi_channel, channel, unique_id } => {
            assert_eq!(dahdi_channel, "1");
            assert_eq!(channel.as_deref(), Some("DAHDI/1-1"));
            assert_eq!(unique_id.as_deref(), Some("u1"));
        }
        other => panic!("expected DAHDIChannel, got {other:?}"),
    }
}

#[test]
fn parse_dahdi_channel_without_channel() {
    let msg = raw(&[("Event", "DAHDIChannel"), ("DAHDIChannel", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DAHDIChannel { channel, unique_id, .. } => {
            assert!(channel.is_none());
            assert!(unique_id.is_none());
        }
        other => panic!("expected DAHDIChannel, got {other:?}"),
    }
}

#[test]
fn parse_alarm() {
    let msg = raw(&[("Event", "Alarm"), ("Alarm", "Red Alarm"), ("Channel", "DAHDI/1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::Alarm { alarm, channel_dahdi, .. } => {
            assert_eq!(alarm, "Red Alarm");
            assert_eq!(channel_dahdi, "DAHDI/1");
        }
        other => panic!("expected Alarm, got {other:?}"),
    }
}

#[test]
fn parse_alarm_clear() {
    let msg = raw(&[("Event", "AlarmClear"), ("Channel", "DAHDI/1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AlarmClear { channel_dahdi, .. } => {
            assert_eq!(channel_dahdi, "DAHDI/1");
        }
        other => panic!("expected AlarmClear, got {other:?}"),
    }
}

#[test]
fn parse_span_alarm() {
    let msg = raw(&[("Event", "SpanAlarm"), ("Span", "1"), ("Alarm", "Red Alarm")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::SpanAlarm { span, alarm, .. } => {
            assert_eq!(span, 1);
            assert_eq!(alarm, "Red Alarm");
        }
        other => panic!("expected SpanAlarm, got {other:?}"),
    }
}

#[test]
fn parse_span_alarm_clear() {
    let msg = raw(&[("Event", "SpanAlarmClear"), ("Span", "2")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::SpanAlarmClear { span, .. } => {
            assert_eq!(span, 2);
        }
        other => panic!("expected SpanAlarmClear, got {other:?}"),
    }
}

// ── aoc ──

#[test]
fn parse_aoc_d() {
    let msg = raw(&[("Event", "AOC-D"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ChargeType", "Currency")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AocD { charge_type, .. } => {
            assert_eq!(charge_type, "Currency");
        }
        other => panic!("expected AocD, got {other:?}"),
    }
}

#[test]
fn parse_aoc_e() {
    let msg = raw(&[("Event", "AOC-E"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("ChargeType", "Unit")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AocE { charge_type, .. } => {
            assert_eq!(charge_type, "Unit");
        }
        other => panic!("expected AocE, got {other:?}"),
    }
}

#[test]
fn parse_aoc_s() {
    let msg = raw(&[("Event", "AOC-S"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AocS { channel, .. } => {
            assert_eq!(channel, "PJSIP/100-0001");
        }
        other => panic!("expected AocS, got {other:?}"),
    }
}

// ── fax events ──

#[test]
fn parse_fax_status() {
    let msg = raw(&[("Event", "FAXStatus"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Operation", "receive"), ("Status", "SENDING"), ("LocalStationID", "12345"), ("FileName", "/tmp/fax.tif")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FAXStatus { operation, status, local_station_id, .. } => {
            assert_eq!(operation, "receive");
            assert_eq!(status, "SENDING");
            assert_eq!(local_station_id, "12345");
        }
        other => panic!("expected FAXStatus, got {other:?}"),
    }
}

#[test]
fn parse_receive_fax() {
    let msg = raw(&[("Event", "ReceiveFAX"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("LocalStationID", "12345"), ("RemoteStationID", "67890"), ("PagesTransferred", "3"), ("Resolution", "200x200"), ("FileName", "/tmp/fax.tif")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ReceiveFAX { pages_transferred, remote_station_id, .. } => {
            assert_eq!(pages_transferred, 3);
            assert_eq!(remote_station_id, "67890");
        }
        other => panic!("expected ReceiveFAX, got {other:?}"),
    }
}

#[test]
fn parse_send_fax() {
    let msg = raw(&[("Event", "SendFAX"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("LocalStationID", "12345"), ("RemoteStationID", "67890"), ("PagesTransferred", "2"), ("Resolution", "200x200"), ("FileName", "/tmp/fax.tif")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::SendFAX { pages_transferred, filename, .. } => {
            assert_eq!(pages_transferred, 2);
            assert_eq!(filename, "/tmp/fax.tif");
        }
        other => panic!("expected SendFAX, got {other:?}"),
    }
}

#[test]
fn parse_fax_session() {
    let msg = raw(&[("Event", "FAXSession"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("SessionNumber", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FAXSession { session_number, .. } => {
            assert_eq!(session_number, "1");
        }
        other => panic!("expected FAXSession, got {other:?}"),
    }
}

#[test]
fn parse_fax_sessions_entry() {
    let msg = raw(&[("Event", "FAXSessionsEntry"), ("Channel", "PJSIP/100-0001"), ("SessionNumber", "1"), ("Technology", "SPANDSP"), ("State", "active"), ("Files", "/tmp/fax.tif")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FAXSessionsEntry { technology, state, .. } => {
            assert_eq!(technology, "SPANDSP");
            assert_eq!(state, "active");
        }
        other => panic!("expected FAXSessionsEntry, got {other:?}"),
    }
}

#[test]
fn parse_fax_sessions_complete() {
    let msg = raw(&[("Event", "FAXSessionsComplete"), ("Total", "5")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FAXSessionsComplete { total, .. } => {
            assert_eq!(total, 5);
        }
        other => panic!("expected FAXSessionsComplete, got {other:?}"),
    }
}

#[test]
fn parse_fax_stats() {
    let msg = raw(&[("Event", "FAXStats"), ("CurrentSessions", "2"), ("ReservedSessions", "1"), ("TransmitAttempts", "10"), ("ReceiveAttempts", "8"), ("CompletedFAXes", "15"), ("FailedFAXes", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::FAXStats { current_sessions, reserved_sessions, transmit_attempts, completed_faxes, failed_faxes, .. } => {
            assert_eq!(current_sessions, 2);
            assert_eq!(reserved_sessions, 1);
            assert_eq!(transmit_attempts, 10);
            assert_eq!(completed_faxes, 15);
            assert_eq!(failed_faxes, 3);
        }
        other => panic!("expected FAXStats, got {other:?}"),
    }
}

// ── meetme events ──

#[test]
fn parse_meetme_join() {
    let msg = raw(&[("Event", "MeetmeJoin"), ("Meetme", "100"), ("Usernum", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeJoin { meetme, user_num, .. } => {
            assert_eq!(meetme, "100");
            assert_eq!(user_num, "1");
        }
        other => panic!("expected MeetmeJoin, got {other:?}"),
    }
}

#[test]
fn parse_meetme_leave() {
    let msg = raw(&[("Event", "MeetmeLeave"), ("Meetme", "100"), ("Usernum", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Duration", "120")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeLeave { meetme, duration, .. } => {
            assert_eq!(meetme, "100");
            assert_eq!(duration, 120);
        }
        other => panic!("expected MeetmeLeave, got {other:?}"),
    }
}

#[test]
fn parse_meetme_end() {
    let msg = raw(&[("Event", "MeetmeEnd"), ("Meetme", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeEnd { meetme, .. } => {
            assert_eq!(meetme, "100");
        }
        other => panic!("expected MeetmeEnd, got {other:?}"),
    }
}

#[test]
fn parse_meetme_mute() {
    let msg = raw(&[("Event", "MeetmeMute"), ("Meetme", "100"), ("Usernum", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Status", "on")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeMute { status, .. } => {
            assert_eq!(status, "on");
        }
        other => panic!("expected MeetmeMute, got {other:?}"),
    }
}

#[test]
fn parse_meetme_talking() {
    let msg = raw(&[("Event", "MeetmeTalking"), ("Meetme", "100"), ("Usernum", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Status", "on")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeTalking { status, .. } => {
            assert_eq!(status, "on");
        }
        other => panic!("expected MeetmeTalking, got {other:?}"),
    }
}

#[test]
fn parse_meetme_talk_request() {
    let msg = raw(&[("Event", "MeetmeTalkRequest"), ("Meetme", "100"), ("Usernum", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Status", "on")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeTalkRequest { status, .. } => {
            assert_eq!(status, "on");
        }
        other => panic!("expected MeetmeTalkRequest, got {other:?}"),
    }
}

#[test]
fn parse_meetme_list() {
    let msg = raw(&[("Event", "MeetmeList"), ("Meetme", "100"), ("Usernum", "1"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Admin", "No"), ("Muted", "No"), ("Talking", "Yes")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeList { admin, muted, talking, .. } => {
            assert_eq!(admin, "No");
            assert_eq!(muted, "No");
            assert_eq!(talking, "Yes");
        }
        other => panic!("expected MeetmeList, got {other:?}"),
    }
}

#[test]
fn parse_meetme_list_rooms() {
    let msg = raw(&[("Event", "MeetmeListRooms"), ("Conference", "100"), ("Parties", "5"), ("Marked", "1"), ("Locked", "No")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MeetmeListRooms { parties, marked, .. } => {
            assert_eq!(parties, 5);
            assert_eq!(marked, 1);
        }
        other => panic!("expected MeetmeListRooms, got {other:?}"),
    }
}

// ── list complete markers ──

#[test]
fn parse_device_state_list_complete() {
    let msg = raw(&[("Event", "DeviceStateListComplete"), ("ListItems", "10")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DeviceStateListComplete { items, .. } => {
            assert_eq!(items, 10);
        }
        other => panic!("expected DeviceStateListComplete, got {other:?}"),
    }
}

#[test]
fn parse_extension_state_list_complete() {
    let msg = raw(&[("Event", "ExtensionStateListComplete"), ("ListItems", "20")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ExtensionStateListComplete { items, .. } => {
            assert_eq!(items, 20);
        }
        other => panic!("expected ExtensionStateListComplete, got {other:?}"),
    }
}

#[test]
fn parse_presence_state_list_complete() {
    let msg = raw(&[("Event", "PresenceStateListComplete"), ("ListItems", "5")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::PresenceStateListComplete { items, .. } => {
            assert_eq!(items, 5);
        }
        other => panic!("expected PresenceStateListComplete, got {other:?}"),
    }
}

// ── pjsip detail/list events ──

#[test]
fn parse_aor_detail() {
    let msg = raw(&[("Event", "AorDetail"), ("ObjectName", "100"), ("Contacts", "100/sip:100@192.168.1.10")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AorDetail { object_name, contacts, .. } => {
            assert_eq!(object_name, "100");
            assert_eq!(contacts, "100/sip:100@192.168.1.10");
        }
        other => panic!("expected AorDetail, got {other:?}"),
    }
}

#[test]
fn parse_aor_list() {
    let msg = raw(&[("Event", "AorList"), ("ObjectName", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AorList { object_name, .. } => {
            assert_eq!(object_name, "100");
        }
        other => panic!("expected AorList, got {other:?}"),
    }
}

#[test]
fn parse_aor_list_complete() {
    let msg = raw(&[("Event", "AorListComplete"), ("ListItems", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AorListComplete { items, .. } => {
            assert_eq!(items, 3);
        }
        other => panic!("expected AorListComplete, got {other:?}"),
    }
}

#[test]
fn parse_auth_detail() {
    let msg = raw(&[("Event", "AuthDetail"), ("ObjectName", "100"), ("Username", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AuthDetail { object_name, username, .. } => {
            assert_eq!(object_name, "100");
            assert_eq!(username, "100");
        }
        other => panic!("expected AuthDetail, got {other:?}"),
    }
}

#[test]
fn parse_auth_list() {
    let msg = raw(&[("Event", "AuthList"), ("ObjectName", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AuthList { object_name, .. } => {
            assert_eq!(object_name, "100");
        }
        other => panic!("expected AuthList, got {other:?}"),
    }
}

#[test]
fn parse_auth_list_complete() {
    let msg = raw(&[("Event", "AuthListComplete"), ("ListItems", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::AuthListComplete { items, .. } => {
            assert_eq!(items, 3);
        }
        other => panic!("expected AuthListComplete, got {other:?}"),
    }
}

#[test]
fn parse_contact_list() {
    let msg = raw(&[("Event", "ContactList"), ("URI", "sip:100@192.168.1.10:5060"), ("ContactStatus", "Reachable"), ("AOR", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ContactList { uri, contact_status, .. } => {
            assert_eq!(uri, "sip:100@192.168.1.10:5060");
            assert_eq!(contact_status, "Reachable");
        }
        other => panic!("expected ContactList, got {other:?}"),
    }
}

#[test]
fn parse_contact_list_complete() {
    let msg = raw(&[("Event", "ContactListComplete"), ("ListItems", "3")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ContactListComplete { items, .. } => {
            assert_eq!(items, 3);
        }
        other => panic!("expected ContactListComplete, got {other:?}"),
    }
}

#[test]
fn parse_contact_status_detail() {
    let msg = raw(&[("Event", "ContactStatusDetail"), ("URI", "sip:100@192.168.1.10:5060"), ("ContactStatus", "Reachable"), ("AOR", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ContactStatusDetail { uri, contact_status, .. } => {
            assert_eq!(uri, "sip:100@192.168.1.10:5060");
            assert_eq!(contact_status, "Reachable");
        }
        other => panic!("expected ContactStatusDetail, got {other:?}"),
    }
}

#[test]
fn parse_endpoint_detail() {
    let msg = raw(&[("Event", "EndpointDetail"), ("ObjectName", "100"), ("DeviceState", "Not in use"), ("ActiveChannels", "0")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::EndpointDetail { object_name, device_state, .. } => {
            assert_eq!(object_name, "100");
            assert_eq!(device_state, "Not in use");
        }
        other => panic!("expected EndpointDetail, got {other:?}"),
    }
}

#[test]
fn parse_endpoint_detail_complete() {
    let msg = raw(&[("Event", "EndpointDetailComplete"), ("ListItems", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::EndpointDetailComplete { items, .. } => {
            assert_eq!(items, 1);
        }
        other => panic!("expected EndpointDetailComplete, got {other:?}"),
    }
}

#[test]
fn parse_endpoint_list() {
    let msg = raw(&[("Event", "EndpointList"), ("ObjectName", "100"), ("Transport", "transport-udp"), ("Aor", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::EndpointList { object_name, transport, .. } => {
            assert_eq!(object_name, "100");
            assert_eq!(transport, "transport-udp");
        }
        other => panic!("expected EndpointList, got {other:?}"),
    }
}

#[test]
fn parse_endpoint_list_complete() {
    let msg = raw(&[("Event", "EndpointListComplete"), ("ListItems", "5")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::EndpointListComplete { items, .. } => {
            assert_eq!(items, 5);
        }
        other => panic!("expected EndpointListComplete, got {other:?}"),
    }
}

#[test]
fn parse_identify_detail() {
    let msg = raw(&[("Event", "IdentifyDetail"), ("ObjectName", "100"), ("Endpoint", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::IdentifyDetail { object_name, endpoint, .. } => {
            assert_eq!(object_name, "100");
            assert_eq!(endpoint, "100");
        }
        other => panic!("expected IdentifyDetail, got {other:?}"),
    }
}

#[test]
fn parse_transport_detail() {
    let msg = raw(&[("Event", "TransportDetail"), ("ObjectName", "transport-udp"), ("Protocol", "udp")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::TransportDetail { object_name, protocol, .. } => {
            assert_eq!(object_name, "transport-udp");
            assert_eq!(protocol, "udp");
        }
        other => panic!("expected TransportDetail, got {other:?}"),
    }
}

#[test]
fn parse_resource_list_detail() {
    let msg = raw(&[("Event", "ResourceListDetail"), ("ObjectName", "mylist")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::ResourceListDetail { object_name, .. } => {
            assert_eq!(object_name, "mylist");
        }
        other => panic!("expected ResourceListDetail, got {other:?}"),
    }
}

#[test]
fn parse_inbound_registration_detail() {
    let msg = raw(&[("Event", "InboundRegistrationDetail"), ("ObjectName", "100"), ("Contacts", "sip:100@192.168.1.10")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::InboundRegistrationDetail { object_name, contacts, .. } => {
            assert_eq!(object_name, "100");
            assert_eq!(contacts, "sip:100@192.168.1.10");
        }
        other => panic!("expected InboundRegistrationDetail, got {other:?}"),
    }
}

#[test]
fn parse_outbound_registration_detail() {
    let msg = raw(&[("Event", "OutboundRegistrationDetail"), ("ObjectName", "trunk"), ("ServerUri", "sip:provider.example.com")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::OutboundRegistrationDetail { object_name, server_uri, .. } => {
            assert_eq!(object_name, "trunk");
            assert_eq!(server_uri, "sip:provider.example.com");
        }
        other => panic!("expected OutboundRegistrationDetail, got {other:?}"),
    }
}

#[test]
fn parse_inbound_subscription_detail() {
    let msg = raw(&[("Event", "InboundSubscriptionDetail"), ("ObjectName", "100")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::InboundSubscriptionDetail { object_name, .. } => {
            assert_eq!(object_name, "100");
        }
        other => panic!("expected InboundSubscriptionDetail, got {other:?}"),
    }
}

#[test]
fn parse_outbound_subscription_detail() {
    let msg = raw(&[("Event", "OutboundSubscriptionDetail"), ("ObjectName", "mwi-sub")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::OutboundSubscriptionDetail { object_name, .. } => {
            assert_eq!(object_name, "mwi-sub");
        }
        other => panic!("expected OutboundSubscriptionDetail, got {other:?}"),
    }
}

// ── mwi ──

#[test]
fn parse_mwi_get() {
    let msg = raw(&[("Event", "MWIGet"), ("Mailbox", "100@default"), ("OldMessages", "3"), ("NewMessages", "1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MWIGet { mailbox, old_messages, new_messages, .. } => {
            assert_eq!(mailbox, "100@default");
            assert_eq!(old_messages, 3);
            assert_eq!(new_messages, 1);
        }
        other => panic!("expected MWIGet, got {other:?}"),
    }
}

#[test]
fn parse_mwi_get_complete() {
    let msg = raw(&[("Event", "MWIGetComplete"), ("ListItems", "5")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MWIGetComplete { items, .. } => {
            assert_eq!(items, 5);
        }
        other => panic!("expected MWIGetComplete, got {other:?}"),
    }
}

// ── misc ──

#[test]
fn parse_mini_voicemail() {
    let msg = raw(&[("Event", "MiniVoiceMail"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("Mailbox", "100@default"), ("Counter", "new")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MiniVoiceMail { mailbox, counter, .. } => {
            assert_eq!(mailbox, "100@default");
            assert_eq!(counter, "new");
        }
        other => panic!("expected MiniVoiceMail, got {other:?}"),
    }
}

#[test]
fn parse_dnd_state() {
    let msg = raw(&[("Event", "DNDState"), ("Channel", "PJSIP/100"), ("Status", "enabled")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::DNDState { channel, status, .. } => {
            assert_eq!(channel, "PJSIP/100");
            assert_eq!(status, "enabled");
        }
        other => panic!("expected DNDState, got {other:?}"),
    }
}

#[test]
fn parse_deadlock_start() {
    let msg = raw(&[("Event", "DeadlockStart")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(matches!(event, AmiEvent::DeadlockStart));
}

#[test]
fn parse_mcid() {
    let msg = raw(&[("Event", "MCID"), ("Channel", "PJSIP/100-0001"), ("Uniqueid", "u1"), ("CallerIDNum", "100"), ("CallerIDName", "Alice")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    match event {
        AmiEvent::MCID { caller_id_num, caller_id_name, .. } => {
            assert_eq!(caller_id_num, "100");
            assert_eq!(caller_id_name, "Alice");
        }
        other => panic!("expected MCID, got {other:?}"),
    }
}

// ── unknown / edge cases ──

#[test]
fn parse_unknown_event() {
    let msg = raw(&[("Event", "SomeWeirdEvent"), ("Foo", "bar")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(matches!(event, AmiEvent::Unknown { .. }));
    assert_eq!(event.event_name(), "SomeWeirdEvent");
}

#[test]
fn from_raw_returns_none_for_non_event() {
    let msg = raw(&[("Response", "Success")]);
    assert!(AmiEvent::from_raw(&msg).is_none());
}

#[test]
fn from_raw_returns_none_for_empty_message() {
    let msg = raw(&[]);
    assert!(AmiEvent::from_raw(&msg).is_none());
}

// ── event_name() method ──

#[test]
fn event_name_newchannel() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "Newchannel"), ("Channel", "x"), ("ChannelState", "0"), ("ChannelStateDesc", "Down"), ("CallerIDNum", "x"), ("CallerIDName", "x"), ("Uniqueid", "x"), ("Linkedid", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "Newchannel");
}

#[test]
fn event_name_hangup() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "Hangup"), ("Channel", "x"), ("Uniqueid", "x"), ("Cause", "0"), ("Cause-txt", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "Hangup");
}

#[test]
fn event_name_dtmfbegin() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "DTMFBegin"), ("Channel", "x"), ("Digit", "1"), ("Direction", "r"), ("Uniqueid", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "DTMFBegin");
}

#[test]
fn event_name_dtmfend() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "DTMFEnd"), ("Channel", "x"), ("Digit", "1"), ("DurationMs", "0"), ("Direction", "r"), ("Uniqueid", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "DTMFEnd");
}

#[test]
fn event_name_cel() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "CEL"), ("Channel", "x"), ("Uniqueid", "x"), ("EventName", "x"), ("AccountCode", "x"), ("ApplicationName", "x"), ("ApplicationData", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "CEL");
}

#[test]
fn event_name_aocd() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "AOC-D"), ("Channel", "x"), ("Uniqueid", "x"), ("ChargeType", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "AOC-D");
}

#[test]
fn event_name_aoce() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "AOC-E"), ("Channel", "x"), ("Uniqueid", "x"), ("ChargeType", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "AOC-E");
}

#[test]
fn event_name_aocs() {
    let event = AmiEvent::from_raw(&raw(&[("Event", "AOC-S"), ("Channel", "x"), ("Uniqueid", "x")])).expect("should parse");
    assert_eq!(event.event_name(), "AOC-S");
}

// ── channel() accessor ──

#[test]
fn channel_returns_some_for_channel_event() {
    let msg = raw(&[("Event", "Newchannel"), ("Channel", "PJSIP/100-0001"),
        ("ChannelState", "0"), ("ChannelStateDesc", "Down"),
        ("CallerIDNum", "100"), ("CallerIDName", "Alice"),
        ("Uniqueid", "u1"), ("Linkedid", "u1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert_eq!(event.channel(), Some("PJSIP/100-0001"));
}

#[test]
fn channel_returns_none_for_fully_booted() {
    let msg = raw(&[("Event", "FullyBooted"), ("Status", "Fully Booted")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(event.channel().is_none());
}

#[test]
fn channel_returns_none_for_bridge_create() {
    let msg = raw(&[("Event", "BridgeCreate"), ("BridgeUniqueid", "br-1"), ("BridgeType", "basic")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(event.channel().is_none());
}

#[test]
fn channel_returns_optional_for_agents() {
    let msg = raw(&[("Event", "Agents"), ("Agent", "100"), ("Name", "x"), ("Status", "x")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(event.channel().is_none());
}

#[test]
fn channel_returns_optional_for_dahdi() {
    let msg = raw(&[("Event", "DAHDIChannel"), ("DAHDIChannel", "1"), ("Channel", "DAHDI/1-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert_eq!(event.channel(), Some("DAHDI/1-1"));
}

// ── unique_id() accessor ──

#[test]
fn unique_id_returns_some_for_hangup() {
    let msg = raw(&[("Event", "Hangup"), ("Channel", "x"), ("Uniqueid", "uid-1"),
        ("Cause", "16"), ("Cause-txt", "Normal")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert_eq!(event.unique_id(), Some("uid-1"));
}

#[test]
fn unique_id_returns_none_for_fully_booted() {
    let msg = raw(&[("Event", "FullyBooted"), ("Status", "Fully Booted")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(event.unique_id().is_none());
}

#[test]
fn unique_id_returns_none_for_peer_status() {
    let msg = raw(&[("Event", "PeerStatus"), ("ChannelType", "PJSIP"),
        ("Peer", "PJSIP/100"), ("PeerStatus", "Reachable")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(event.unique_id().is_none());
}

#[test]
fn unique_id_returns_optional_for_user_event() {
    let msg = raw(&[("Event", "UserEvent"), ("UserEvent", "Ping")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert!(event.unique_id().is_none());
}

#[test]
fn unique_id_returns_optional_for_dahdi_channel() {
    let msg = raw(&[("Event", "DAHDIChannel"), ("DAHDIChannel", "1"), ("Uniqueid", "uid-1")]);
    let event = AmiEvent::from_raw(&msg).expect("should parse");
    assert_eq!(event.unique_id(), Some("uid-1"));
}
