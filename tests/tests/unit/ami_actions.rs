#![allow(clippy::unwrap_used)]

use asterisk_rs_ami::action::*;

/// helper to extract a header value from a RawAmiMessage
fn get_header(msg: &asterisk_rs_ami::codec::RawAmiMessage, key: &str) -> Option<String> {
    msg.headers
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

/// helper to collect all values for a given header key
fn get_all_headers(msg: &asterisk_rs_ami::codec::RawAmiMessage, key: &str) -> Vec<String> {
    msg.headers
        .iter()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .collect()
}

#[test]
fn next_action_id_returns_unique_ids() {
    let id1 = next_action_id();
    let id2 = next_action_id();
    let id3 = next_action_id();
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
}

#[test]
fn next_action_id_is_numeric() {
    let id = next_action_id();
    assert!(
        id.parse::<u64>().is_ok(),
        "action ID should be a numeric string"
    );
}

#[test]
fn logoff_action_headers() {
    let action = LogoffAction;
    assert_eq!(action.action_name(), "Logoff");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Logoff".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn ping_action_headers() {
    let action = PingAction;
    assert_eq!(action.action_name(), "Ping");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Ping".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn core_status_action_headers() {
    let action = CoreStatusAction;
    assert_eq!(action.action_name(), "CoreStatus");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("CoreStatus".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn core_settings_action_headers() {
    let action = CoreSettingsAction;
    assert_eq!(action.action_name(), "CoreSettings");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("CoreSettings".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn core_show_channels_action_headers() {
    let action = CoreShowChannelsAction;
    assert_eq!(action.action_name(), "CoreShowChannels");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("CoreShowChannels".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn list_commands_action_headers() {
    let action = ListCommandsAction;
    assert_eq!(action.action_name(), "ListCommands");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ListCommands".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn logger_rotate_action_headers() {
    let action = LoggerRotateAction;
    assert_eq!(action.action_name(), "LoggerRotate");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("LoggerRotate".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn bridge_list_action_headers() {
    let action = BridgeListAction;
    assert_eq!(action.action_name(), "BridgeList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("BridgeList".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn confbridge_list_rooms_action_headers() {
    let action = ConfbridgeListRoomsAction;
    assert_eq!(action.action_name(), "ConfbridgeListRooms");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("ConfbridgeListRooms".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn parkinglots_action_headers() {
    let action = ParkinglotsAction;
    assert_eq!(action.action_name(), "Parkinglots");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Parkinglots".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_endpoints_action_headers() {
    let action = PJSIPShowEndpointsAction;
    assert_eq!(action.action_name(), "PJSIPShowEndpoints");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowEndpoints".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_registrations_inbound_action_headers() {
    let action = PJSIPShowRegistrationsInboundAction;
    assert_eq!(action.action_name(), "PJSIPShowRegistrationsInbound");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowRegistrationsInbound".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_registrations_outbound_action_headers() {
    let action = PJSIPShowRegistrationsOutboundAction;
    assert_eq!(action.action_name(), "PJSIPShowRegistrationsOutbound");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowRegistrationsOutbound".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_contacts_action_headers() {
    let action = PJSIPShowContactsAction;
    assert_eq!(action.action_name(), "PJSIPShowContacts");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPShowContacts".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_aors_action_headers() {
    let action = PJSIPShowAorsAction;
    assert_eq!(action.action_name(), "PJSIPShowAors");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPShowAors".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_auths_action_headers() {
    let action = PJSIPShowAuthsAction;
    assert_eq!(action.action_name(), "PJSIPShowAuths");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPShowAuths".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn extension_state_list_action_headers() {
    let action = ExtensionStateListAction;
    assert_eq!(action.action_name(), "ExtensionStateList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("ExtensionStateList".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn device_state_list_action_headers() {
    let action = DeviceStateListAction;
    assert_eq!(action.action_name(), "DeviceStateList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DeviceStateList".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn presence_state_list_action_headers() {
    let action = PresenceStateListAction;
    assert_eq!(action.action_name(), "PresenceStateList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PresenceStateList".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn voicemail_users_list_action_headers() {
    let action = VoicemailUsersListAction;
    assert_eq!(action.action_name(), "VoicemailUsersList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("VoicemailUsersList".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn meetme_list_rooms_action_headers() {
    let action = MeetmeListRoomsAction;
    assert_eq!(action.action_name(), "MeetmeListRooms");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MeetmeListRooms".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn agents_action_headers() {
    let action = AgentsAction;
    assert_eq!(action.action_name(), "Agents");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Agents".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn fax_sessions_action_headers() {
    let action = FAXSessionsAction;
    assert_eq!(action.action_name(), "FAXSessions");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("FAXSessions".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn fax_stats_action_headers() {
    let action = FAXStatsAction;
    assert_eq!(action.action_name(), "FAXStats");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("FAXStats".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn dahdi_restart_action_headers() {
    let action = DAHDIRestartAction;
    assert_eq!(action.action_name(), "DAHDIRestart");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDIRestart".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn dahdi_show_status_action_headers() {
    let action = DAHDIShowStatusAction;
    assert_eq!(action.action_name(), "DAHDIShowStatus");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDIShowStatus".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn iax_netstats_action_headers() {
    let action = IAXnetstatsAction;
    assert_eq!(action.action_name(), "IAXnetstats");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("IAXnetstats".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn iax_peerlist_action_headers() {
    let action = IAXpeerlistAction;
    assert_eq!(action.action_name(), "IAXpeerlist");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("IAXpeerlist".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn iax_peers_action_headers() {
    let action = IAXpeersAction;
    assert_eq!(action.action_name(), "IAXpeers");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("IAXpeers".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn iax_registry_action_headers() {
    let action = IAXregistryAction;
    assert_eq!(action.action_name(), "IAXregistry");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("IAXregistry".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pri_debug_file_unset_action_headers() {
    let action = PRIDebugFileUnsetAction;
    assert_eq!(action.action_name(), "PRIDebugFileUnset");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PRIDebugFileUnset".into()));
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn bridge_technology_list_action_headers() {
    let action = BridgeTechnologyListAction;
    assert_eq!(action.action_name(), "BridgeTechnologyList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("BridgeTechnologyList".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_reg_inbound_contact_statuses_action_headers() {
    let action = PJSIPShowRegistrationInboundContactStatusesAction;
    assert_eq!(
        action.action_name(),
        "PJSIPShowRegistrationInboundContactStatuses"
    );
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowRegistrationInboundContactStatuses".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_resource_lists_action_headers() {
    let action = PJSIPShowResourceListsAction;
    assert_eq!(action.action_name(), "PJSIPShowResourceLists");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowResourceLists".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_subscriptions_inbound_action_headers() {
    let action = PJSIPShowSubscriptionsInboundAction;
    assert_eq!(action.action_name(), "PJSIPShowSubscriptionsInbound");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowSubscriptionsInbound".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn pjsip_show_subscriptions_outbound_action_headers() {
    let action = PJSIPShowSubscriptionsOutboundAction;
    assert_eq!(action.action_name(), "PJSIPShowSubscriptionsOutbound");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("PJSIPShowSubscriptionsOutbound".into())
    );
    assert_eq!(get_header(&msg, "ActionID"), Some(id));
}

#[test]
fn challenge_action_headers() {
    let action = ChallengeAction;
    assert_eq!(action.action_name(), "Challenge");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Challenge".into()));
    assert_eq!(get_header(&msg, "AuthType"), Some("md5".into()));
}

#[test]
fn login_action_headers() {
    let action = LoginAction::new("admin", "pass");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Login".into()));
    assert_eq!(get_header(&msg, "Username"), Some("admin".into()));
    assert_eq!(get_header(&msg, "Secret"), Some("pass".into()));
}

#[test]
fn challenge_login_action_headers() {
    let action = ChallengeLoginAction {
        username: "admin".into(),
        key: "abc123".into(),
    };
    assert_eq!(action.action_name(), "Login");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Action"), Some("Login".into()));
    assert_eq!(get_header(&msg, "AuthType"), Some("md5".into()));
    assert_eq!(get_header(&msg, "Username"), Some("admin".into()));
    assert_eq!(get_header(&msg, "Key"), Some("abc123".into()));
}

#[test]
fn core_show_channel_map_action_headers() {
    let action = CoreShowChannelMapAction {
        channel: "SIP/100-0001".into(),
    };
    assert_eq!(action.action_name(), "CoreShowChannelMap");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("CoreShowChannelMap".into())
    );
    assert_eq!(get_header(&msg, "Channel"), Some("SIP/100-0001".into()));
}

#[test]
fn events_action_headers() {
    let action = EventsAction {
        event_mask: "call,agent".into(),
    };
    assert_eq!(action.action_name(), "Events");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Events".into()));
    assert_eq!(get_header(&msg, "EventMask"), Some("call,agent".into()));
}

#[test]
fn wait_event_action_headers() {
    let action = WaitEventAction { timeout: 30 };
    assert_eq!(action.action_name(), "WaitEvent");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("WaitEvent".into()));
    assert_eq!(get_header(&msg, "Timeout"), Some("30".into()));
}

#[test]
fn module_check_action_headers() {
    let action = ModuleCheckAction {
        module: "chan_pjsip.so".into(),
    };
    assert_eq!(action.action_name(), "ModuleCheck");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ModuleCheck".into()));
    assert_eq!(get_header(&msg, "Module"), Some("chan_pjsip.so".into()));
}

#[test]
fn module_load_action_headers() {
    let action = ModuleLoadAction {
        module: "chan_pjsip.so".into(),
        load_type: "load".into(),
    };
    assert_eq!(action.action_name(), "ModuleLoad");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ModuleLoad".into()));
    assert_eq!(get_header(&msg, "Module"), Some("chan_pjsip.so".into()));
    assert_eq!(get_header(&msg, "LoadType"), Some("load".into()));
}

#[test]
fn absolute_timeout_action_headers() {
    let action = AbsoluteTimeoutAction {
        channel: "PJSIP/100-0001".into(),
        timeout: 60,
    };
    assert_eq!(action.action_name(), "AbsoluteTimeout");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("AbsoluteTimeout".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Timeout"), Some("60".into()));
}

#[test]
fn mute_audio_action_headers() {
    let action = MuteAudioAction {
        channel: "PJSIP/100-0001".into(),
        direction: "both".into(),
        state: "on".into(),
    };
    assert_eq!(action.action_name(), "MuteAudio");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MuteAudio".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Direction"), Some("both".into()));
    assert_eq!(get_header(&msg, "State"), Some("on".into()));
}

#[test]
fn send_text_action_headers() {
    let action = SendTextAction {
        channel: "PJSIP/100-0001".into(),
        message: "hello world".into(),
    };
    assert_eq!(action.action_name(), "SendText");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("SendText".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Message"), Some("hello world".into()));
}

#[test]
fn db_get_action_headers() {
    let action = DBGetAction {
        family: "cidname".into(),
        key: "12125551234".into(),
    };
    assert_eq!(action.action_name(), "DBGet");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DBGet".into()));
    assert_eq!(get_header(&msg, "Family"), Some("cidname".into()));
    assert_eq!(get_header(&msg, "Key"), Some("12125551234".into()));
}

#[test]
fn db_put_action_headers() {
    let action = DBPutAction {
        family: "cidname".into(),
        key: "12125551234".into(),
        val: "John Doe".into(),
    };
    assert_eq!(action.action_name(), "DBPut");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DBPut".into()));
    assert_eq!(get_header(&msg, "Family"), Some("cidname".into()));
    assert_eq!(get_header(&msg, "Key"), Some("12125551234".into()));
    assert_eq!(get_header(&msg, "Val"), Some("John Doe".into()));
}

#[test]
fn db_del_action_headers() {
    let action = DBDelAction {
        family: "cidname".into(),
        key: "12125551234".into(),
    };
    assert_eq!(action.action_name(), "DBDel");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DBDel".into()));
    assert_eq!(get_header(&msg, "Family"), Some("cidname".into()));
    assert_eq!(get_header(&msg, "Key"), Some("12125551234".into()));
}

#[test]
fn atxfer_action_headers() {
    let action = AtxferAction {
        channel: "PJSIP/100-0001".into(),
        exten: "200".into(),
        context: "default".into(),
    };
    assert_eq!(action.action_name(), "Atxfer");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Atxfer".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Exten"), Some("200".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
}

#[test]
fn blind_transfer_action_headers() {
    let action = BlindTransferAction {
        channel: "PJSIP/100-0001".into(),
        exten: "200".into(),
        context: "default".into(),
    };
    assert_eq!(action.action_name(), "BlindTransfer");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("BlindTransfer".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Exten"), Some("200".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
}

#[test]
fn cancel_atxfer_action_headers() {
    let action = CancelAtxferAction {
        channel: "PJSIP/100-0001".into(),
    };
    assert_eq!(action.action_name(), "CancelAtxfer");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("CancelAtxfer".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
}

#[test]
fn bridge_destroy_action_headers() {
    let action = BridgeDestroyAction {
        bridge_unique_id: "abc-123".into(),
    };
    assert_eq!(action.action_name(), "BridgeDestroy");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("BridgeDestroy".into()));
    assert_eq!(get_header(&msg, "BridgeUniqueid"), Some("abc-123".into()));
}

#[test]
fn bridge_info_action_headers() {
    let action = BridgeInfoAction {
        bridge_unique_id: "abc-123".into(),
    };
    assert_eq!(action.action_name(), "BridgeInfo");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("BridgeInfo".into()));
    assert_eq!(get_header(&msg, "BridgeUniqueid"), Some("abc-123".into()));
}

#[test]
fn bridge_kick_action_headers() {
    let action = BridgeKickAction {
        bridge_unique_id: "abc-123".into(),
        channel: "PJSIP/100-0001".into(),
    };
    assert_eq!(action.action_name(), "BridgeKick");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("BridgeKick".into()));
    assert_eq!(get_header(&msg, "BridgeUniqueid"), Some("abc-123".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
}

#[test]
fn queue_remove_action_headers() {
    let action = QueueRemoveAction {
        queue: "support".into(),
        interface: "PJSIP/100".into(),
    };
    assert_eq!(action.action_name(), "QueueRemove");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("QueueRemove".into()));
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Interface"), Some("PJSIP/100".into()));
}

#[test]
fn mix_monitor_mute_action_headers() {
    let action = MixMonitorMuteAction {
        channel: "PJSIP/100-0001".into(),
        direction: "both".into(),
        state: "1".into(),
    };
    assert_eq!(action.action_name(), "MixMonitorMute");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MixMonitorMute".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Direction"), Some("both".into()));
    assert_eq!(get_header(&msg, "State"), Some("1".into()));
}

#[test]
fn control_playback_action_headers() {
    let action = ControlPlaybackAction {
        channel: "PJSIP/100-0001".into(),
        control: "pause".into(),
    };
    assert_eq!(action.action_name(), "ControlPlayback");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ControlPlayback".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Control"), Some("pause".into()));
}

#[test]
fn confbridge_list_action_headers() {
    let action = ConfbridgeListAction {
        conference: "100".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeList");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ConfbridgeList".into()));
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
}

#[test]
fn confbridge_kick_action_headers() {
    let action = ConfbridgeKickAction {
        conference: "100".into(),
        channel: "PJSIP/200-0001".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeKick");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ConfbridgeKick".into()));
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/200-0001".into()));
}

#[test]
fn confbridge_mute_action_headers() {
    let action = ConfbridgeMuteAction {
        conference: "100".into(),
        channel: "PJSIP/200-0001".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeMute");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ConfbridgeMute".into()));
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/200-0001".into()));
}

#[test]
fn confbridge_unmute_action_headers() {
    let action = ConfbridgeUnmuteAction {
        conference: "100".into(),
        channel: "PJSIP/200-0001".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeUnmute");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ConfbridgeUnmute".into()));
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/200-0001".into()));
}

#[test]
fn confbridge_lock_action_headers() {
    let action = ConfbridgeLockAction {
        conference: "100".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeLock");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ConfbridgeLock".into()));
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
}

#[test]
fn confbridge_unlock_action_headers() {
    let action = ConfbridgeUnlockAction {
        conference: "100".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeUnlock");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ConfbridgeUnlock".into()));
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
}

#[test]
fn confbridge_stop_record_action_headers() {
    let action = ConfbridgeStopRecordAction {
        conference: "100".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeStopRecord");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("ConfbridgeStopRecord".into())
    );
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
}

#[test]
fn confbridge_set_single_video_src_action_headers() {
    let action = ConfbridgeSetSingleVideoSrcAction {
        conference: "100".into(),
        channel: "PJSIP/200-0001".into(),
    };
    assert_eq!(action.action_name(), "ConfbridgeSetSingleVideoSrc");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("ConfbridgeSetSingleVideoSrc".into())
    );
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/200-0001".into()));
}

#[test]
fn get_config_json_action_headers() {
    let action = GetConfigJSONAction {
        filename: "sip.conf".into(),
    };
    assert_eq!(action.action_name(), "GetConfigJSON");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("GetConfigJSON".into()));
    assert_eq!(get_header(&msg, "Filename"), Some("sip.conf".into()));
}

#[test]
fn create_config_action_headers() {
    let action = CreateConfigAction {
        filename: "test.conf".into(),
    };
    assert_eq!(action.action_name(), "CreateConfig");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("CreateConfig".into()));
    assert_eq!(get_header(&msg, "Filename"), Some("test.conf".into()));
}

#[test]
fn list_categories_action_headers() {
    let action = ListCategoriesAction {
        filename: "sip.conf".into(),
    };
    assert_eq!(action.action_name(), "ListCategories");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ListCategories".into()));
    assert_eq!(get_header(&msg, "Filename"), Some("sip.conf".into()));
}

#[test]
fn pjsip_show_endpoint_action_headers() {
    let action = PJSIPShowEndpointAction {
        endpoint: "100".into(),
    };
    assert_eq!(action.action_name(), "PJSIPShowEndpoint");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPShowEndpoint".into()));
    assert_eq!(get_header(&msg, "Endpoint"), Some("100".into()));
}

#[test]
fn pjsip_qualify_action_headers() {
    let action = PJSIPQualifyAction {
        endpoint: "100".into(),
    };
    assert_eq!(action.action_name(), "PJSIPQualify");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPQualify".into()));
    assert_eq!(get_header(&msg, "Endpoint"), Some("100".into()));
}

#[test]
fn pjsip_register_action_headers() {
    let action = PJSIPRegisterAction {
        registration: "trunk1".into(),
    };
    assert_eq!(action.action_name(), "PJSIPRegister");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPRegister".into()));
    assert_eq!(get_header(&msg, "Registration"), Some("trunk1".into()));
}

#[test]
fn pjsip_unregister_action_headers() {
    let action = PJSIPUnregisterAction {
        registration: "trunk1".into(),
    };
    assert_eq!(action.action_name(), "PJSIPUnregister");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PJSIPUnregister".into()));
    assert_eq!(get_header(&msg, "Registration"), Some("trunk1".into()));
}

#[test]
fn extension_state_action_headers() {
    let action = ExtensionStateAction {
        exten: "100".into(),
        context: "default".into(),
    };
    assert_eq!(action.action_name(), "ExtensionState");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("ExtensionState".into()));
    assert_eq!(get_header(&msg, "Exten"), Some("100".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
}

#[test]
fn presence_state_action_headers() {
    let action = PresenceStateAction {
        provider: "CustomPresence:100".into(),
    };
    assert_eq!(action.action_name(), "PresenceState");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PresenceState".into()));
    assert_eq!(
        get_header(&msg, "Provider"),
        Some("CustomPresence:100".into())
    );
}

#[test]
fn dialplan_extension_add_action_headers() {
    let action = DialplanExtensionAddAction {
        context: "default".into(),
        extension: "100".into(),
        priority: "1".into(),
        application: "Dial".into(),
    };
    assert_eq!(action.action_name(), "DialplanExtensionAdd");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("DialplanExtensionAdd".into())
    );
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Extension"), Some("100".into()));
    assert_eq!(get_header(&msg, "Priority"), Some("1".into()));
    assert_eq!(get_header(&msg, "Application"), Some("Dial".into()));
}

#[test]
fn local_optimize_away_action_headers() {
    let action = LocalOptimizeAwayAction {
        channel: "Local/100@default-0001".into(),
    };
    assert_eq!(action.action_name(), "LocalOptimizeAway");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("LocalOptimizeAway".into()));
    assert_eq!(
        get_header(&msg, "Channel"),
        Some("Local/100@default-0001".into())
    );
}

#[test]
fn mailbox_count_action_headers() {
    let action = MailboxCountAction {
        mailbox: "100@default".into(),
    };
    assert_eq!(action.action_name(), "MailboxCount");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MailboxCount".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100@default".into()));
}

#[test]
fn mailbox_status_action_headers() {
    let action = MailboxStatusAction {
        mailbox: "100@default".into(),
    };
    assert_eq!(action.action_name(), "MailboxStatus");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MailboxStatus".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100@default".into()));
}

#[test]
fn mwi_get_action_headers() {
    let action = MWIGetAction {
        mailbox: "100@default".into(),
    };
    assert_eq!(action.action_name(), "MWIGet");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MWIGet".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100@default".into()));
}

#[test]
fn mwi_delete_action_headers() {
    let action = MWIDeleteAction {
        mailbox: "100@default".into(),
    };
    assert_eq!(action.action_name(), "MWIDelete");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MWIDelete".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100@default".into()));
}

#[test]
fn voicemail_user_status_action_headers() {
    let action = VoicemailUserStatusAction {
        context: "default".into(),
        mailbox: "100".into(),
    };
    assert_eq!(action.action_name(), "VoicemailUserStatus");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("VoicemailUserStatus".into())
    );
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100".into()));
}

#[test]
fn voicemail_box_summary_action_headers() {
    let action = VoicemailBoxSummaryAction {
        context: "default".into(),
        mailbox: "100".into(),
    };
    assert_eq!(action.action_name(), "VoicemailBoxSummary");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("VoicemailBoxSummary".into())
    );
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100".into()));
}

#[test]
fn meetme_mute_action_headers() {
    let action = MeetmeMuteAction {
        meetme: "100".into(),
        usernum: "1".into(),
    };
    assert_eq!(action.action_name(), "MeetmeMute");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MeetmeMute".into()));
    assert_eq!(get_header(&msg, "Meetme"), Some("100".into()));
    assert_eq!(get_header(&msg, "Usernum"), Some("1".into()));
}

#[test]
fn meetme_unmute_action_headers() {
    let action = MeetmeUnmuteAction {
        meetme: "100".into(),
        usernum: "1".into(),
    };
    assert_eq!(action.action_name(), "MeetmeUnmute");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("MeetmeUnmute".into()));
    assert_eq!(get_header(&msg, "Meetme"), Some("100".into()));
    assert_eq!(get_header(&msg, "Usernum"), Some("1".into()));
}

#[test]
fn fax_session_action_headers() {
    let action = FAXSessionAction {
        session_number: "42".into(),
    };
    assert_eq!(action.action_name(), "FAXSession");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("FAXSession".into()));
    assert_eq!(get_header(&msg, "SessionNumber"), Some("42".into()));
}

#[test]
fn aoc_message_action_headers() {
    let action = AOCMessageAction {
        channel: "PJSIP/100-0001".into(),
        msg_type: "D".into(),
        charge_type: "Currency".into(),
    };
    assert_eq!(action.action_name(), "AOCMessage");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("AOCMessage".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "MsgType"), Some("D".into()));
    assert_eq!(get_header(&msg, "ChargeType"), Some("Currency".into()));
}

#[test]
fn send_flash_action_headers() {
    let action = SendFlashAction {
        channel: "DAHDI/1-1".into(),
    };
    assert_eq!(action.action_name(), "SendFlash");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("SendFlash".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("DAHDI/1-1".into()));
}

#[test]
fn dahdi_dnd_off_action_headers() {
    let action = DAHDIDNDoffAction {
        dahdi_channel: "1".into(),
    };
    assert_eq!(action.action_name(), "DAHDIDNDoff");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDIDNDoff".into()));
    assert_eq!(get_header(&msg, "DAHDIChannel"), Some("1".into()));
}

#[test]
fn dahdi_dnd_on_action_headers() {
    let action = DAHDIDNDonAction {
        dahdi_channel: "1".into(),
    };
    assert_eq!(action.action_name(), "DAHDIDNDon");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDIDNDon".into()));
    assert_eq!(get_header(&msg, "DAHDIChannel"), Some("1".into()));
}

#[test]
fn dahdi_dial_offhook_action_headers() {
    let action = DAHDIDialOffhookAction {
        dahdi_channel: "1".into(),
        number: "5551234".into(),
    };
    assert_eq!(action.action_name(), "DAHDIDialOffhook");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDIDialOffhook".into()));
    assert_eq!(get_header(&msg, "DAHDIChannel"), Some("1".into()));
    assert_eq!(get_header(&msg, "Number"), Some("5551234".into()));
}

#[test]
fn dahdi_hangup_action_headers() {
    let action = DAHDIHangupAction {
        dahdi_channel: "1".into(),
    };
    assert_eq!(action.action_name(), "DAHDIHangup");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDIHangup".into()));
    assert_eq!(get_header(&msg, "DAHDIChannel"), Some("1".into()));
}

#[test]
fn dahdi_transfer_action_headers() {
    let action = DAHDITransferAction {
        dahdi_channel: "1".into(),
    };
    assert_eq!(action.action_name(), "DAHDITransfer");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("DAHDITransfer".into()));
    assert_eq!(get_header(&msg, "DAHDIChannel"), Some("1".into()));
}

#[test]
fn pri_debug_file_set_action_headers() {
    let action = PRIDebugFileSetAction {
        filename: "/tmp/pri.log".into(),
    };
    assert_eq!(action.action_name(), "PRIDebugFileSet");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PRIDebugFileSet".into()));
    assert_eq!(get_header(&msg, "Filename"), Some("/tmp/pri.log".into()));
}

#[test]
fn pri_debug_set_action_headers() {
    let action = PRIDebugSetAction { span: 1, level: 4 };
    assert_eq!(action.action_name(), "PRIDebugSet");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("PRIDebugSet".into()));
    assert_eq!(get_header(&msg, "Span"), Some("1".into()));
    assert_eq!(get_header(&msg, "Level"), Some("4".into()));
}

#[test]
fn bridge_technology_suspend_action_headers() {
    let action = BridgeTechnologySuspendAction {
        bridge_technology: "simple_bridge".into(),
    };
    assert_eq!(action.action_name(), "BridgeTechnologySuspend");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("BridgeTechnologySuspend".into())
    );
    assert_eq!(
        get_header(&msg, "BridgeTechnology"),
        Some("simple_bridge".into())
    );
}

#[test]
fn bridge_technology_unsuspend_action_headers() {
    let action = BridgeTechnologyUnsuspendAction {
        bridge_technology: "simple_bridge".into(),
    };
    assert_eq!(action.action_name(), "BridgeTechnologyUnsuspend");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("BridgeTechnologyUnsuspend".into())
    );
    assert_eq!(
        get_header(&msg, "BridgeTechnology"),
        Some("simple_bridge".into())
    );
}

#[test]
fn queue_change_priority_caller_action_headers() {
    let action = QueueChangePriorityCallerAction {
        queue: "support".into(),
        caller: "PJSIP/100-0001".into(),
        priority: 5,
    };
    assert_eq!(action.action_name(), "QueueChangePriorityCaller");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("QueueChangePriorityCaller".into())
    );
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Caller"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Priority"), Some("5".into()));
}

#[test]
fn queue_member_ring_in_use_action_headers() {
    let action = QueueMemberRingInUseAction {
        interface: "PJSIP/100".into(),
        ring_in_use: true,
    };
    assert_eq!(action.action_name(), "QueueMemberRingInUse");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("QueueMemberRingInUse".into())
    );
    assert_eq!(get_header(&msg, "Interface"), Some("PJSIP/100".into()));
    assert_eq!(get_header(&msg, "RingInUse"), Some("true".into()));
}

#[test]
fn queue_rule_action_headers() {
    let action = QueueRuleAction {
        rule: "myrule".into(),
    };
    assert_eq!(action.action_name(), "QueueRule");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("QueueRule".into()));
    assert_eq!(get_header(&msg, "Rule"), Some("myrule".into()));
}

#[test]
fn queue_withdraw_caller_action_headers() {
    let action = QueueWithdrawCallerAction {
        queue: "support".into(),
        caller: "PJSIP/100-0001".into(),
    };
    assert_eq!(action.action_name(), "QueueWithdrawCaller");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("QueueWithdrawCaller".into())
    );
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Caller"), Some("PJSIP/100-0001".into()));
}

#[test]
fn sorcery_memory_cache_expire_action_headers() {
    let action = SorceryMemoryCacheExpireAction {
        cache: "contacts".into(),
    };
    assert_eq!(action.action_name(), "SorceryMemoryCacheExpire");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("SorceryMemoryCacheExpire".into())
    );
    assert_eq!(get_header(&msg, "Cache"), Some("contacts".into()));
}

#[test]
fn sorcery_memory_cache_expire_object_action_headers() {
    let action = SorceryMemoryCacheExpireObjectAction {
        cache: "contacts".into(),
        object: "obj-1".into(),
    };
    assert_eq!(action.action_name(), "SorceryMemoryCacheExpireObject");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("SorceryMemoryCacheExpireObject".into())
    );
    assert_eq!(get_header(&msg, "Cache"), Some("contacts".into()));
    assert_eq!(get_header(&msg, "Object"), Some("obj-1".into()));
}

#[test]
fn sorcery_memory_cache_populate_action_headers() {
    let action = SorceryMemoryCachePopulateAction {
        cache: "contacts".into(),
    };
    assert_eq!(action.action_name(), "SorceryMemoryCachePopulate");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("SorceryMemoryCachePopulate".into())
    );
    assert_eq!(get_header(&msg, "Cache"), Some("contacts".into()));
}

#[test]
fn sorcery_memory_cache_stale_action_headers() {
    let action = SorceryMemoryCacheStaleAction {
        cache: "contacts".into(),
    };
    assert_eq!(action.action_name(), "SorceryMemoryCacheStale");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("SorceryMemoryCacheStale".into())
    );
    assert_eq!(get_header(&msg, "Cache"), Some("contacts".into()));
}

#[test]
fn sorcery_memory_cache_stale_object_action_headers() {
    let action = SorceryMemoryCacheStaleObjectAction {
        cache: "contacts".into(),
        object: "obj-1".into(),
    };
    assert_eq!(action.action_name(), "SorceryMemoryCacheStaleObject");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(
        get_header(&msg, "Action"),
        Some("SorceryMemoryCacheStaleObject".into())
    );
    assert_eq!(get_header(&msg, "Cache"), Some("contacts".into()));
    assert_eq!(get_header(&msg, "Object"), Some("obj-1".into()));
}

#[test]
fn redirect_action_headers() {
    let action = RedirectAction {
        channel: "PJSIP/100-0001".into(),
        context: "default".into(),
        exten: "200".into(),
        priority: 1,
    };
    assert_eq!(action.action_name(), "Redirect");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Redirect".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Exten"), Some("200".into()));
    assert_eq!(get_header(&msg, "Priority"), Some("1".into()));
}

#[test]
fn jabber_send_action_headers() {
    let action = JabberSendAction {
        jabber: "asterisk".into(),
        jid: "user@example.com".into(),
        message: "hello".into(),
    };
    assert_eq!(action.action_name(), "JabberSend");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("JabberSend".into()));
    assert_eq!(get_header(&msg, "Jabber"), Some("asterisk".into()));
    assert_eq!(get_header(&msg, "JID"), Some("user@example.com".into()));
    assert_eq!(get_header(&msg, "Message"), Some("hello".into()));
}

#[test]
fn command_action_headers() {
    let action = CommandAction::new("core show channels");
    assert_eq!(action.action_name(), "Command");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Action"), Some("Command".into()));
    assert_eq!(
        get_header(&msg, "Command"),
        Some("core show channels".into())
    );
}

#[test]
fn command_action_direct_construction() {
    let action = CommandAction {
        command: "sip show peers".into(),
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Command"), Some("sip show peers".into()));
}

#[test]
fn get_var_action_with_channel() {
    let action = GetVarAction {
        channel: Some("PJSIP/100-0001".into()),
        variable: "CALLERID(num)".into(),
    };
    assert_eq!(action.action_name(), "GetVar");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Variable"), Some("CALLERID(num)".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
}

#[test]
fn get_var_action_without_channel() {
    let action = GetVarAction {
        channel: None,
        variable: "EPOCH".into(),
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Variable"), Some("EPOCH".into()));
    assert_eq!(get_header(&msg, "Channel"), None);
}

#[test]
fn set_var_action_with_channel() {
    let action = SetVarAction {
        channel: Some("PJSIP/100-0001".into()),
        variable: "MY_VAR".into(),
        value: "my_value".into(),
    };
    assert_eq!(action.action_name(), "SetVar");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Variable"), Some("MY_VAR".into()));
    assert_eq!(get_header(&msg, "Value"), Some("my_value".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
}

#[test]
fn set_var_action_without_channel() {
    let action = SetVarAction {
        channel: None,
        variable: "GLOBAL_VAR".into(),
        value: "42".into(),
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), None);
}

#[test]
fn status_action_with_channel() {
    let action = StatusAction {
        channel: Some("PJSIP/100-0001".into()),
    };
    assert_eq!(action.action_name(), "Status");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
}

#[test]
fn status_action_without_channel() {
    let action = StatusAction { channel: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), None);
}

#[test]
fn filter_action_with_filter() {
    let action = FilterAction {
        operation: "Add".into(),
        filter: Some("Event: Newchannel".into()),
    };
    assert_eq!(action.action_name(), "Filter");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Operation"), Some("Add".into()));
    assert_eq!(get_header(&msg, "Filter"), Some("Event: Newchannel".into()));
}

#[test]
fn filter_action_without_filter() {
    let action = FilterAction {
        operation: "Show".into(),
        filter: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Filter"), None);
}

#[test]
fn reload_action_with_module() {
    let action = ReloadAction {
        module: Some("chan_pjsip.so".into()),
    };
    assert_eq!(action.action_name(), "Reload");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Module"), Some("chan_pjsip.so".into()));
}

#[test]
fn reload_action_without_module() {
    let action = ReloadAction { module: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Module"), None);
}

#[test]
fn user_event_action_headers() {
    let action = UserEventAction {
        user_event: "MyEvent".into(),
        headers: vec![
            ("Key1".into(), "Val1".into()),
            ("Key2".into(), "Val2".into()),
        ],
    };
    assert_eq!(action.action_name(), "UserEvent");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "UserEvent"), Some("MyEvent".into()));
    assert_eq!(get_header(&msg, "Key1"), Some("Val1".into()));
    assert_eq!(get_header(&msg, "Key2"), Some("Val2".into()));
}

#[test]
fn user_event_action_no_extra_headers() {
    let action = UserEventAction {
        user_event: "Simple".into(),
        headers: vec![],
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "UserEvent"), Some("Simple".into()));
}

#[test]
fn play_dtmf_action_with_duration() {
    let action = PlayDTMFAction {
        channel: "PJSIP/100-0001".into(),
        digit: "5".into(),
        duration: Some(250),
    };
    assert_eq!(action.action_name(), "PlayDTMF");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Digit"), Some("5".into()));
    assert_eq!(get_header(&msg, "Duration"), Some("250".into()));
}

#[test]
fn play_dtmf_action_without_duration() {
    let action = PlayDTMFAction {
        channel: "PJSIP/100-0001".into(),
        digit: "#".into(),
        duration: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Duration"), None);
}

#[test]
fn agi_action_with_command_id() {
    let action = AGIAction {
        channel: "PJSIP/100-0001".into(),
        command: "EXEC Playback hello-world".into(),
        command_id: Some("cmd-1".into()),
    };
    assert_eq!(action.action_name(), "AGI");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(
        get_header(&msg, "Command"),
        Some("EXEC Playback hello-world".into())
    );
    assert_eq!(get_header(&msg, "CommandID"), Some("cmd-1".into()));
}

#[test]
fn agi_action_without_command_id() {
    let action = AGIAction {
        channel: "PJSIP/100-0001".into(),
        command: "ANSWER".into(),
        command_id: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "CommandID"), None);
}

#[test]
fn db_del_tree_action_with_key() {
    let action = DBDelTreeAction {
        family: "cidname".into(),
        key: Some("123".into()),
    };
    assert_eq!(action.action_name(), "DBDelTree");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Family"), Some("cidname".into()));
    assert_eq!(get_header(&msg, "Key"), Some("123".into()));
}

#[test]
fn db_del_tree_action_without_key() {
    let action = DBDelTreeAction {
        family: "cidname".into(),
        key: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Key"), None);
}

#[test]
fn db_get_tree_action_with_key() {
    let action = DBGetTreeAction {
        family: "cidname".into(),
        key: Some("123".into()),
    };
    assert_eq!(action.action_name(), "DBGetTree");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Family"), Some("cidname".into()));
    assert_eq!(get_header(&msg, "Key"), Some("123".into()));
}

#[test]
fn db_get_tree_action_without_key() {
    let action = DBGetTreeAction {
        family: "cidname".into(),
        key: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Key"), None);
}

#[test]
fn bridge_action_with_tone() {
    let action = BridgeAction {
        channel1: "PJSIP/100-0001".into(),
        channel2: "PJSIP/200-0001".into(),
        tone: Some("yes".into()),
    };
    assert_eq!(action.action_name(), "Bridge");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel1"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Channel2"), Some("PJSIP/200-0001".into()));
    assert_eq!(get_header(&msg, "Tone"), Some("yes".into()));
}

#[test]
fn bridge_action_without_tone() {
    let action = BridgeAction {
        channel1: "PJSIP/100-0001".into(),
        channel2: "PJSIP/200-0001".into(),
        tone: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Tone"), None);
}

#[test]
fn queue_pause_action_headers() {
    let action = QueuePauseAction {
        queue: Some("support".into()),
        interface: "PJSIP/100".into(),
        paused: true,
        reason: Some("break".into()),
    };
    assert_eq!(action.action_name(), "QueuePause");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Interface"), Some("PJSIP/100".into()));
    assert_eq!(get_header(&msg, "Paused"), Some("true".into()));
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Reason"), Some("break".into()));
}

#[test]
fn queue_pause_action_minimal() {
    let action = QueuePauseAction {
        queue: None,
        interface: "PJSIP/100".into(),
        paused: false,
        reason: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), None);
    assert_eq!(get_header(&msg, "Reason"), None);
    assert_eq!(get_header(&msg, "Paused"), Some("false".into()));
}

#[test]
fn queue_penalty_action_with_queue() {
    let action = QueuePenaltyAction {
        interface: "PJSIP/100".into(),
        penalty: 3,
        queue: Some("support".into()),
    };
    assert_eq!(action.action_name(), "QueuePenalty");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Interface"), Some("PJSIP/100".into()));
    assert_eq!(get_header(&msg, "Penalty"), Some("3".into()));
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
}

#[test]
fn queue_penalty_action_without_queue() {
    let action = QueuePenaltyAction {
        interface: "PJSIP/100".into(),
        penalty: 0,
        queue: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), None);
}

#[test]
fn queue_status_action_all_fields() {
    let action = QueueStatusAction {
        queue: Some("support".into()),
        member: Some("PJSIP/100".into()),
    };
    assert_eq!(action.action_name(), "QueueStatus");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Member"), Some("PJSIP/100".into()));
}

#[test]
fn queue_status_action_empty() {
    let action = QueueStatusAction {
        queue: None,
        member: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), None);
    assert_eq!(get_header(&msg, "Member"), None);
}

#[test]
fn queue_summary_action_with_queue() {
    let action = QueueSummaryAction {
        queue: Some("support".into()),
    };
    assert_eq!(action.action_name(), "QueueSummary");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
}

#[test]
fn queue_summary_action_without_queue() {
    let action = QueueSummaryAction { queue: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), None);
}

#[test]
fn queue_reload_action_all_fields() {
    let action = QueueReloadAction {
        queue: Some("support".into()),
        members: Some("yes".into()),
        rules: Some("yes".into()),
        parameters: Some("yes".into()),
    };
    assert_eq!(action.action_name(), "QueueReload");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Members"), Some("yes".into()));
    assert_eq!(get_header(&msg, "Rules"), Some("yes".into()));
    assert_eq!(get_header(&msg, "Parameters"), Some("yes".into()));
}

#[test]
fn queue_reload_action_empty() {
    let action = QueueReloadAction {
        queue: None,
        members: None,
        rules: None,
        parameters: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), None);
    assert_eq!(get_header(&msg, "Members"), None);
}

#[test]
fn queue_reset_action_with_queue() {
    let action = QueueResetAction {
        queue: Some("support".into()),
    };
    assert_eq!(action.action_name(), "QueueReset");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
}

#[test]
fn queue_reset_action_without_queue() {
    let action = QueueResetAction { queue: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), None);
}

#[test]
fn queue_log_action_all_fields() {
    let action = QueueLogAction {
        queue: "support".into(),
        event: "ABANDON".into(),
        interface: Some("PJSIP/100".into()),
        unique_id: Some("1234.5".into()),
        message: Some("caller gave up".into()),
    };
    assert_eq!(action.action_name(), "QueueLog");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Event"), Some("ABANDON".into()));
    assert_eq!(get_header(&msg, "Interface"), Some("PJSIP/100".into()));
    assert_eq!(get_header(&msg, "UniqueID"), Some("1234.5".into()));
    assert_eq!(get_header(&msg, "Message"), Some("caller gave up".into()));
}

#[test]
fn queue_log_action_minimal() {
    let action = QueueLogAction {
        queue: "support".into(),
        event: "CUSTOM".into(),
        interface: None,
        unique_id: None,
        message: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Interface"), None);
    assert_eq!(get_header(&msg, "UniqueID"), None);
    assert_eq!(get_header(&msg, "Message"), None);
}

#[test]
fn stop_mix_monitor_action_with_id() {
    let action = StopMixMonitorAction {
        channel: "PJSIP/100-0001".into(),
        mix_monitor_id: Some("abc-123".into()),
    };
    assert_eq!(action.action_name(), "StopMixMonitor");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "MixMonitorID"), Some("abc-123".into()));
}

#[test]
fn stop_mix_monitor_action_without_id() {
    let action = StopMixMonitorAction {
        channel: "PJSIP/100-0001".into(),
        mix_monitor_id: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "MixMonitorID"), None);
}

#[test]
fn confbridge_start_record_action_with_file() {
    let action = ConfbridgeStartRecordAction {
        conference: "100".into(),
        record_file: Some("/tmp/conf.wav".into()),
    };
    assert_eq!(action.action_name(), "ConfbridgeStartRecord");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
    assert_eq!(get_header(&msg, "RecordFile"), Some("/tmp/conf.wav".into()));
}

#[test]
fn confbridge_start_record_action_without_file() {
    let action = ConfbridgeStartRecordAction {
        conference: "100".into(),
        record_file: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "RecordFile"), None);
}

#[test]
fn parked_calls_action_with_lot() {
    let action = ParkedCallsAction {
        parking_lot: Some("default".into()),
    };
    assert_eq!(action.action_name(), "ParkedCalls");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "ParkingLot"), Some("default".into()));
}

#[test]
fn parked_calls_action_without_lot() {
    let action = ParkedCallsAction { parking_lot: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "ParkingLot"), None);
}

#[test]
fn get_config_action_with_category() {
    let action = GetConfigAction {
        filename: "sip.conf".into(),
        category: Some("general".into()),
    };
    assert_eq!(action.action_name(), "GetConfig");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Filename"), Some("sip.conf".into()));
    assert_eq!(get_header(&msg, "Category"), Some("general".into()));
}

#[test]
fn get_config_action_without_category() {
    let action = GetConfigAction {
        filename: "sip.conf".into(),
        category: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Category"), None);
}

#[test]
fn update_config_action_headers() {
    let action = UpdateConfigAction {
        src_filename: "sip.conf".into(),
        dst_filename: "sip.conf".into(),
        reload: Some("chan_sip.so".into()),
        actions: vec![("NewCat".into(), "my-endpoint".into())],
    };
    assert_eq!(action.action_name(), "UpdateConfig");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "SrcFilename"), Some("sip.conf".into()));
    assert_eq!(get_header(&msg, "DstFilename"), Some("sip.conf".into()));
    assert_eq!(get_header(&msg, "Reload"), Some("chan_sip.so".into()));
    assert_eq!(get_header(&msg, "Action-000000"), Some("NewCat".into()));
    assert_eq!(get_header(&msg, "Cat-000000"), Some("my-endpoint".into()));
}

#[test]
fn update_config_action_no_reload() {
    let action = UpdateConfigAction {
        src_filename: "a.conf".into(),
        dst_filename: "b.conf".into(),
        reload: None,
        actions: vec![],
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Reload"), None);
}

#[test]
fn show_dial_plan_action_all_fields() {
    let action = ShowDialPlanAction {
        extension: Some("100".into()),
        context: Some("default".into()),
    };
    assert_eq!(action.action_name(), "ShowDialPlan");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Extension"), Some("100".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
}

#[test]
fn show_dial_plan_action_empty() {
    let action = ShowDialPlanAction {
        extension: None,
        context: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Extension"), None);
    assert_eq!(get_header(&msg, "Context"), None);
}

#[test]
fn pjsip_notify_action_headers() {
    let action = PJSIPNotifyAction {
        endpoint: "100".into(),
        variable: vec![("Event".into(), "check-sync".into())],
    };
    assert_eq!(action.action_name(), "PJSIPNotify");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Endpoint"), Some("100".into()));
    let vars = get_all_headers(&msg, "Variable");
    assert_eq!(vars, vec!["Event=check-sync"]);
}

#[test]
fn pjsip_notify_action_no_variables() {
    let action = PJSIPNotifyAction {
        endpoint: "100".into(),
        variable: vec![],
    };
    let (_, msg) = action.to_message();
    assert!(get_all_headers(&msg, "Variable").is_empty());
}

#[test]
fn pjsip_hangup_action_with_cause() {
    let action = PJSIPHangupAction {
        channel: "PJSIP/100-0001".into(),
        cause: Some(16),
    };
    assert_eq!(action.action_name(), "PJSIPHangup");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Cause"), Some("16".into()));
}

#[test]
fn pjsip_hangup_action_without_cause() {
    let action = PJSIPHangupAction {
        channel: "PJSIP/100-0001".into(),
        cause: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Cause"), None);
}

#[test]
fn dialplan_extension_remove_action_with_priority() {
    let action = DialplanExtensionRemoveAction {
        context: "default".into(),
        extension: "100".into(),
        priority: Some("1".into()),
    };
    assert_eq!(action.action_name(), "DialplanExtensionRemove");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Extension"), Some("100".into()));
    assert_eq!(get_header(&msg, "Priority"), Some("1".into()));
}

#[test]
fn dialplan_extension_remove_action_without_priority() {
    let action = DialplanExtensionRemoveAction {
        context: "default".into(),
        extension: "100".into(),
        priority: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Priority"), None);
}

#[test]
fn mwi_update_action_all_fields() {
    let action = MWIUpdateAction {
        mailbox: "100@default".into(),
        old_messages: Some(3),
        new_messages: Some(1),
    };
    assert_eq!(action.action_name(), "MWIUpdate");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Mailbox"), Some("100@default".into()));
    assert_eq!(get_header(&msg, "OldMessages"), Some("3".into()));
    assert_eq!(get_header(&msg, "NewMessages"), Some("1".into()));
}

#[test]
fn mwi_update_action_minimal() {
    let action = MWIUpdateAction {
        mailbox: "100@default".into(),
        old_messages: None,
        new_messages: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "OldMessages"), None);
    assert_eq!(get_header(&msg, "NewMessages"), None);
}

#[test]
fn message_send_action_all_fields() {
    let action = MessageSendAction {
        to: "pjsip:100@default".into(),
        from: Some("pjsip:200@default".into()),
        body: Some("hello".into()),
    };
    assert_eq!(action.action_name(), "MessageSend");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "To"), Some("pjsip:100@default".into()));
    assert_eq!(get_header(&msg, "From"), Some("pjsip:200@default".into()));
    assert_eq!(get_header(&msg, "Body"), Some("hello".into()));
}

#[test]
fn message_send_action_minimal() {
    let action = MessageSendAction {
        to: "pjsip:100@default".into(),
        from: None,
        body: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "From"), None);
    assert_eq!(get_header(&msg, "Body"), None);
}

#[test]
fn voicemail_refresh_action_all() {
    let action = VoicemailRefreshAction {
        context: Some("default".into()),
        mailbox: Some("100".into()),
    };
    assert_eq!(action.action_name(), "VoicemailRefresh");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Mailbox"), Some("100".into()));
}

#[test]
fn voicemail_refresh_action_empty() {
    let action = VoicemailRefreshAction {
        context: None,
        mailbox: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Context"), None);
    assert_eq!(get_header(&msg, "Mailbox"), None);
}

#[test]
fn meetme_list_action_with_conference() {
    let action = MeetmeListAction {
        conference: Some("100".into()),
    };
    assert_eq!(action.action_name(), "MeetmeList");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Conference"), Some("100".into()));
}

#[test]
fn meetme_list_action_without_conference() {
    let action = MeetmeListAction { conference: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Conference"), None);
}

#[test]
fn agent_logoff_action_with_soft() {
    let action = AgentLogoffAction {
        agent: "1001".into(),
        soft: Some(true),
    };
    assert_eq!(action.action_name(), "AgentLogoff");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Agent"), Some("1001".into()));
    assert_eq!(get_header(&msg, "Soft"), Some("true".into()));
}

#[test]
fn agent_logoff_action_without_soft() {
    let action = AgentLogoffAction {
        agent: "1001".into(),
        soft: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Soft"), None);
}

#[test]
fn play_mf_action_with_duration() {
    let action = PlayMFAction {
        channel: "PJSIP/100-0001".into(),
        digit: "5".into(),
        duration: Some(100),
    };
    assert_eq!(action.action_name(), "PlayMF");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Digit"), Some("5".into()));
    assert_eq!(get_header(&msg, "Duration"), Some("100".into()));
}

#[test]
fn play_mf_action_without_duration() {
    let action = PlayMFAction {
        channel: "PJSIP/100-0001".into(),
        digit: "5".into(),
        duration: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Duration"), None);
}

#[test]
fn dahdi_show_channels_action_with_channel() {
    let action = DAHDIShowChannelsAction {
        dahdi_channel: Some("1".into()),
    };
    assert_eq!(action.action_name(), "DAHDIShowChannels");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "DAHDIChannel"), Some("1".into()));
}

#[test]
fn dahdi_show_channels_action_without_channel() {
    let action = DAHDIShowChannelsAction {
        dahdi_channel: None,
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "DAHDIChannel"), None);
}

#[test]
fn pri_show_spans_action_with_span() {
    let action = PRIShowSpansAction { span: Some(1) };
    assert_eq!(action.action_name(), "PRIShowSpans");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Span"), Some("1".into()));
}

#[test]
fn pri_show_spans_action_without_span() {
    let action = PRIShowSpansAction { span: None };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Span"), None);
}

#[test]
fn voicemail_forward_action_headers() {
    let action = VoicemailForwardAction {
        mailbox: "200".into(),
        context: "default".into(),
        from_mailbox: "100".into(),
        from_context: "default".into(),
        from_folder: "INBOX".into(),
        message_id: "msg-001".into(),
    };
    assert_eq!(action.action_name(), "VoicemailForward");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Mailbox"), Some("200".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "FromMailbox"), Some("100".into()));
    assert_eq!(get_header(&msg, "FromContext"), Some("default".into()));
    assert_eq!(get_header(&msg, "FromFolder"), Some("INBOX".into()));
    assert_eq!(get_header(&msg, "ID"), Some("msg-001".into()));
}

#[test]
fn voicemail_move_action_headers() {
    let action = VoicemailMoveAction {
        mailbox: "100".into(),
        context: "default".into(),
        folder: "Old".into(),
        message_id: "msg-001".into(),
    };
    assert_eq!(action.action_name(), "VoicemailMove");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Mailbox"), Some("100".into()));
    assert_eq!(get_header(&msg, "Folder"), Some("Old".into()));
    assert_eq!(get_header(&msg, "ID"), Some("msg-001".into()));
}

#[test]
fn voicemail_remove_action_headers() {
    let action = VoicemailRemoveAction {
        mailbox: "100".into(),
        context: "default".into(),
        folder: "INBOX".into(),
        message_id: "msg-002".into(),
    };
    assert_eq!(action.action_name(), "VoicemailRemove");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Mailbox"), Some("100".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Folder"), Some("INBOX".into()));
    assert_eq!(get_header(&msg, "ID"), Some("msg-002".into()));
}

// ---------------------------------------------------------------------------
// originate action builder tests
// ---------------------------------------------------------------------------

#[test]
fn originate_action_minimal() {
    let action = OriginateAction::new("PJSIP/100");
    assert_eq!(action.action_name(), "Originate");
    let (id, msg) = action.to_message();
    assert!(!id.is_empty());
    assert_eq!(get_header(&msg, "Action"), Some("Originate".into()));
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100".into()));
    // optional fields should be absent
    assert_eq!(get_header(&msg, "Context"), None);
    assert_eq!(get_header(&msg, "Exten"), None);
    assert_eq!(get_header(&msg, "Priority"), None);
    assert_eq!(get_header(&msg, "Application"), None);
    assert_eq!(get_header(&msg, "Data"), None);
    assert_eq!(get_header(&msg, "Timeout"), None);
    assert_eq!(get_header(&msg, "CallerID"), None);
    assert_eq!(get_header(&msg, "Account"), None);
    assert_eq!(get_header(&msg, "Async"), None);
}

#[test]
fn originate_action_context_flow() {
    let action = OriginateAction::new("PJSIP/100")
        .context("default")
        .extension("200")
        .priority(1);
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100".into()));
    assert_eq!(get_header(&msg, "Context"), Some("default".into()));
    assert_eq!(get_header(&msg, "Exten"), Some("200".into()));
    assert_eq!(get_header(&msg, "Priority"), Some("1".into()));
}

#[test]
fn originate_action_application_flow() {
    let action = OriginateAction::new("PJSIP/100")
        .application("Playback")
        .data("hello-world");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Application"), Some("Playback".into()));
    assert_eq!(get_header(&msg, "Data"), Some("hello-world".into()));
}

#[test]
fn originate_action_all_options() {
    let action = OriginateAction::new("PJSIP/100")
        .context("from-internal")
        .extension("200")
        .priority(1)
        .timeout_ms(30000)
        .caller_id("\"John\" <100>")
        .account("billing-1")
        .async_originate(true)
        .variable("CDR(accountcode)", "12345")
        .variable("CHANNEL(language)", "en");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Timeout"), Some("30000".into()));
    assert_eq!(get_header(&msg, "Account"), Some("billing-1".into()));
    assert_eq!(get_header(&msg, "Async"), Some("true".into()));
    let vars = get_all_headers(&msg, "Variable");
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], "CDR(accountcode)=12345");
    assert_eq!(vars[1], "CHANNEL(language)=en");
}

#[test]
fn originate_action_async_false_omits_header() {
    let action = OriginateAction::new("PJSIP/100").async_originate(false);
    let (_, msg) = action.to_message();
    // async_ is false by default, so Async header should not be present
    assert_eq!(get_header(&msg, "Async"), None);
}

#[test]
fn originate_action_direct_construction() {
    let action = OriginateAction {
        channel: "SIP/trunk/5551234".into(),
        context: Some("outbound".into()),
        exten: Some("s".into()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(45000),
        caller_id: Some("5559999".into()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let (_, msg) = action.to_message();
    assert_eq!(
        get_header(&msg, "Channel"),
        Some("SIP/trunk/5551234".into())
    );
    assert_eq!(get_header(&msg, "CallerID"), Some("5559999".into()));
    assert_eq!(get_header(&msg, "Async"), Some("true".into()));
}

// ---------------------------------------------------------------------------
// hangup action builder tests
// ---------------------------------------------------------------------------

#[test]
fn hangup_action_minimal() {
    let action = HangupAction::new("PJSIP/100-0001");
    assert_eq!(action.action_name(), "Hangup");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Cause"), None);
}

#[test]
fn hangup_action_with_cause() {
    let action = HangupAction::new("PJSIP/100-0001").cause(16);
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Cause"), Some("16".into()));
}

#[test]
fn hangup_action_direct_construction() {
    let action = HangupAction {
        channel: "DAHDI/1-1".into(),
        cause: Some(21),
    };
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("DAHDI/1-1".into()));
    assert_eq!(get_header(&msg, "Cause"), Some("21".into()));
}

// ---------------------------------------------------------------------------
// queue add action builder tests
// ---------------------------------------------------------------------------

#[test]
fn queue_add_action_minimal() {
    let action = QueueAddAction::new("support", "PJSIP/100");
    assert_eq!(action.action_name(), "QueueAdd");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Queue"), Some("support".into()));
    assert_eq!(get_header(&msg, "Interface"), Some("PJSIP/100".into()));
    assert_eq!(get_header(&msg, "Penalty"), None);
    assert_eq!(get_header(&msg, "Paused"), None);
    assert_eq!(get_header(&msg, "MemberName"), None);
    assert_eq!(get_header(&msg, "StateInterface"), None);
}

#[test]
fn queue_add_action_all_options() {
    let action = QueueAddAction::new("support", "PJSIP/100")
        .penalty(5)
        .paused(true)
        .member_name("Agent 100")
        .state_interface("PJSIP/100");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Penalty"), Some("5".into()));
    assert_eq!(get_header(&msg, "Paused"), Some("true".into()));
    assert_eq!(get_header(&msg, "MemberName"), Some("Agent 100".into()));
    assert_eq!(get_header(&msg, "StateInterface"), Some("PJSIP/100".into()));
}

// ---------------------------------------------------------------------------
// mix monitor action builder tests
// ---------------------------------------------------------------------------

#[test]
fn mix_monitor_action_minimal() {
    let action = MixMonitorAction::new("PJSIP/100-0001");
    assert_eq!(action.action_name(), "MixMonitor");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "File"), None);
    assert_eq!(get_header(&msg, "Options"), None);
}

#[test]
fn mix_monitor_action_all_options() {
    let action = MixMonitorAction::new("PJSIP/100-0001")
        .file("/tmp/recording.wav")
        .options("r");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "File"), Some("/tmp/recording.wav".into()));
    assert_eq!(get_header(&msg, "Options"), Some("r".into()));
}

// ---------------------------------------------------------------------------
// park action builder tests
// ---------------------------------------------------------------------------

#[test]
fn park_action_minimal() {
    let action = ParkAction::new("PJSIP/100-0001");
    assert_eq!(action.action_name(), "Park");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Channel"), Some("PJSIP/100-0001".into()));
    assert_eq!(get_header(&msg, "Timeout"), None);
    assert_eq!(get_header(&msg, "AnnounceChannel"), None);
    assert_eq!(get_header(&msg, "ParkingLot"), None);
}

#[test]
fn park_action_all_options() {
    let action = ParkAction::new("PJSIP/100-0001")
        .timeout(30)
        .announce_channel("PJSIP/200-0001")
        .parking_lot("default");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Timeout"), Some("30".into()));
    assert_eq!(
        get_header(&msg, "AnnounceChannel"),
        Some("PJSIP/200-0001".into())
    );
    assert_eq!(get_header(&msg, "ParkingLot"), Some("default".into()));
}

// ---------------------------------------------------------------------------
// to_message contract tests
// ---------------------------------------------------------------------------

#[test]
fn to_message_always_includes_action_header() {
    let action = PingAction;
    let (_, msg) = action.to_message();
    assert_eq!(msg.headers[0], ("Action".to_string(), "Ping".to_string()));
}

#[test]
fn to_message_always_includes_action_id_header() {
    let action = PingAction;
    let (id, msg) = action.to_message();
    assert_eq!(msg.headers[1], ("ActionID".to_string(), id));
}

#[test]
fn to_message_output_and_channel_variables_empty() {
    let action = PingAction;
    let (_, msg) = action.to_message();
    assert!(msg.output.is_empty());
    assert!(msg.channel_variables.is_empty());
}

#[test]
fn login_action_new_creates_correctly() {
    let action = LoginAction::new("admin", "secret123");
    assert_eq!(action.username, "admin");
    assert_eq!(action.secret(), "secret123");
}

#[test]
fn login_action_debug_redacts_secret() {
    let action = LoginAction::new("admin", "super_secret_password");
    let debug_output = format!("{:?}", action);
    assert!(
        !debug_output.contains("super_secret_password"),
        "debug output must not contain the secret: {debug_output}"
    );
    assert!(
        debug_output.contains("[REDACTED]"),
        "debug output should contain [REDACTED]: {debug_output}"
    );
    assert!(
        debug_output.contains("admin"),
        "debug output should contain username: {debug_output}"
    );
}

#[test]
fn login_action_new_headers_match() {
    let action = LoginAction::new("testuser", "testpass");
    let (_, msg) = action.to_message();
    assert_eq!(get_header(&msg, "Action"), Some("Login".into()));
    assert_eq!(get_header(&msg, "Username"), Some("testuser".into()));
    assert_eq!(get_header(&msg, "Secret"), Some("testpass".into()));
}
