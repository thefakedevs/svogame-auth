mod common;

use common::TestApp;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn user_can_create_squad_and_see_it_in_profile() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderOne", false, &[]).await;

    let squad = app.create_squad(&leader, "Alpha Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");

    let my_squad_response = app.get_json("/api/user/me/squad", &leader.access_token).await;
    assert!(my_squad_response.status().is_success());
    let my_squad: serde_json::Value = my_squad_response.json().await.expect("my squad json");

    assert_eq!(my_squad["id"], squad_id);
    assert_eq!(my_squad["memberCount"], 1);
    assert_eq!(app.user_squad_id(&leader.user_id).await.as_deref(), Some(squad_id));
    assert_eq!(app.audit_log_count("user.squad.created").await, 1);
}

#[tokio::test]
#[serial]
async fn create_squad_restriction_blocks_squad_creation() {
    let app = TestApp::spawn().await;
    let restricted_user = app
        .issue_user_token("RestrictedCreator", false, &["create_squad"])
        .await;

    let response = app
        .post_json(
            "/api/squads",
            &restricted_user.access_token,
            serde_json::json!({ "name": "Blocked Squad" }),
        )
        .await;

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(app.audit_log_count("user.squad.created").await, 0);
}

#[tokio::test]
#[serial]
async fn leader_can_invite_and_user_can_accept_invite() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderTwo", false, &[]).await;
    let invited = app.issue_user_token("InvitedUser", false, &[]).await;

    let squad = app.create_squad(&leader, "Bravo Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;
    assert_eq!(app.invite_count_for_squad(squad_id).await, 1);

    let response = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &invited.access_token,
            serde_json::json!({}),
        )
        .await;

    assert!(response.status().is_success());
    assert_eq!(app.user_squad_id(&invited.user_id).await.as_deref(), Some(squad_id));
    assert_eq!(app.invite_count_for_squad(squad_id).await, 0);
    assert_eq!(app.audit_log_count("user.squad.invite_created").await, 1);
    assert_eq!(app.audit_log_count("user.squad.invite_accepted").await, 1);
}

#[tokio::test]
#[serial]
async fn expired_invite_cannot_be_accepted() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderThree", false, &[]).await;
    let invited = app.issue_user_token("LateUser", false, &[]).await;

    let squad = app.create_squad(&leader, "Charlie Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;
    app.expire_invite(&invite_id).await;

    let response = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &invited.access_token,
            serde_json::json!({}),
        )
        .await;

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(app.user_squad_id(&invited.user_id).await, None);
}

#[tokio::test]
#[serial]
async fn join_squad_restriction_blocks_invite_acceptance() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderFour", false, &[]).await;
    let invited = app
        .issue_user_token("RestrictedJoiner", false, &["join_squad"])
        .await;

    let squad = app.create_squad(&leader, "Delta Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;

    let response = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &invited.access_token,
            serde_json::json!({}),
        )
        .await;

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(app.user_squad_id(&invited.user_id).await, None);
}

#[tokio::test]
#[serial]
async fn leader_cannot_leave_squad_but_can_disband_it() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderFive", false, &[]).await;
    let member = app.issue_user_token("MemberFive", false, &[]).await;

    let squad = app.create_squad(&leader, "Echo Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();
    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_id}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let leave_response = app
        .post_json(
            &format!("/api/squads/{squad_id}/leave"),
            &leader.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(leave_response.status(), reqwest::StatusCode::FORBIDDEN);

    let delete_response = app.delete(&format!("/api/squads/{squad_id}"), &leader.access_token).await;
    assert!(delete_response.status().is_success());
    assert!(!app.squad_exists(&squad_id).await);
    assert_eq!(app.user_squad_id(&leader.user_id).await, None);
    assert_eq!(app.user_squad_id(&member.user_id).await, None);
    assert_eq!(app.audit_log_count("user.squad.disbanded").await, 1);
}

#[tokio::test]
#[serial]
async fn restricted_squad_blocks_new_members_until_admin_unrestricts_it() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("AdminUser", true, &[]).await;
    let leader = app.issue_user_token("LeaderSix", false, &[]).await;
    let member = app.issue_user_token("MemberSix", false, &[]).await;

    let squad = app.create_squad(&leader, "Foxtrot Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();

    let restrict_response = app
        .post_json(
            &format!("/api/admin/squads/{squad_id}/restrict"),
            &admin.access_token,
            serde_json::json!({ "reason": "Under moderation" }),
        )
        .await;
    assert!(restrict_response.status().is_success());

    let invite_response = app
        .post_json(
            &format!("/api/squads/{squad_id}/invites"),
            &leader.access_token,
            serde_json::json!({ "userId": member.user_id }),
        )
        .await;
    assert_eq!(invite_response.status(), reqwest::StatusCode::FORBIDDEN);

    let unrestrict_response = app
        .post_json(
            &format!("/api/admin/squads/{squad_id}/unrestrict"),
            &admin.access_token,
            serde_json::json!({ "reason": "Approved" }),
        )
        .await;
    assert!(unrestrict_response.status().is_success());

    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    let accept_response = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(accept_response.status().is_success());
}

#[tokio::test]
#[serial]
async fn admin_can_kick_member_from_squad() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("AdminKick", true, &[]).await;
    let leader = app.issue_user_token("LeaderSeven", false, &[]).await;
    let member = app.issue_user_token("MemberSeven", false, &[]).await;

    let squad = app.create_squad(&leader, "Golf Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();
    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_id}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let response = app
        .post_json(
            &format!("/api/admin/squads/{squad_id}/members/{}/kick", member.user_id),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;

    assert!(response.status().is_success());
    assert_eq!(app.user_squad_id(&member.user_id).await, None);
    assert_eq!(app.audit_log_count("admin.squad.member_kicked").await, 1);
}

#[tokio::test]
#[serial]
async fn reset_auth_epoch_revokes_existing_token() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("AdminReset", true, &[]).await;
    let user = app.issue_user_token("VictimUser", false, &[]).await;

    let before_response = app.get_json("/api/user/me", &user.access_token).await;
    assert!(before_response.status().is_success());

    let reset_response = app
        .post_json(
            &format!("/api/admin/users/{}/reset-auth-epoch", user.user_id),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(reset_response.status().is_success());

    let after_response = app.get_json("/api/user/me", &user.access_token).await;
    assert_eq!(after_response.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(app.audit_log_count("admin.user.auth_epoch_reset").await, 1);
}

#[tokio::test]
#[serial]
async fn debug_issue_token_can_seed_superuser_and_restrictions() {
    let app = TestApp::spawn().await;

    let issued = app
        .issue_user_token("FlowInspector", true, &["create_squad", "join_squad"])
        .await;

    let me_response = app.get_json("/api/user/me", &issued.access_token).await;
    assert!(me_response.status().is_success());
    let me: serde_json::Value = me_response.json().await.expect("me json");

    let restrictions = app.my_restrictions(&issued).await;
    let keys: Vec<_> = restrictions
        .iter()
        .map(|restriction| restriction["key"].as_str().expect("restriction key"))
        .collect();

    assert_eq!(me["username"], "FlowInspector");
    assert_eq!(me["isSuperuser"], true);
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"create_squad"));
    assert!(keys.contains(&"join_squad"));
}

#[tokio::test]
#[serial]
async fn admin_can_grant_and_revoke_restrictions_and_user_sees_them() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("RestrictionAdmin", true, &[]).await;
    let user = app.issue_user_token("RestrictedTarget", false, &[]).await;

    let grant_response = app
        .grant_user_restriction(&admin, &user.user_id, "join_squad", "Blocked from squads")
        .await;
    assert!(grant_response.status().is_success());

    let restrictions_after_grant = app.my_restrictions(&user).await;
    assert_eq!(restrictions_after_grant.len(), 1);
    assert_eq!(restrictions_after_grant[0]["key"], "join_squad");
    assert_eq!(restrictions_after_grant[0]["reason"], "Blocked from squads");

    let revoke_response = app
        .revoke_user_restriction(&admin, &user.user_id, "join_squad", "Restriction removed")
        .await;
    assert!(revoke_response.status().is_success());

    let restrictions_after_revoke = app.my_restrictions(&user).await;
    assert!(restrictions_after_revoke.is_empty());
    assert_eq!(app.audit_log_count("admin.user.restriction_granted").await, 1);
    assert_eq!(app.audit_log_count("admin.user.restriction_revoked").await, 1);
}

#[tokio::test]
#[serial]
async fn admin_can_search_users_by_username_and_uuid() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SearchAdmin", true, &[]).await;
    let searched_user = app.issue_user_token("UniqueSearchTarget", false, &[]).await;
    let _another_user = app.issue_user_token("AnotherSearchTarget", false, &[]).await;

    let username_search = app
        .get_json("/api/admin/users?q=UniqueSearchTarget&page=1&perPage=10", &admin.access_token)
        .await;
    assert!(username_search.status().is_success());
    let username_search_body: serde_json::Value =
        username_search.json().await.expect("username search json");

    assert_eq!(username_search_body["total"], 1);
    assert_eq!(username_search_body["items"][0]["id"], searched_user.user_id);
    assert_eq!(username_search_body["items"][0]["username"], "UniqueSearchTarget");

    let uuid_search = app
        .get_json(
            &format!("/api/admin/users?q={}&page=1&perPage=10", searched_user.user_id),
            &admin.access_token,
        )
        .await;
    assert!(uuid_search.status().is_success());
    let uuid_search_body: serde_json::Value = uuid_search.json().await.expect("uuid search json");

    assert_eq!(uuid_search_body["total"], 1);
    assert_eq!(uuid_search_body["items"][0]["id"], searched_user.user_id);
}

#[tokio::test]
#[serial]
async fn admin_can_patch_user_profile_fields() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("PatchAdmin", true, &[]).await;
    let user = app.issue_user_token("OriginalUser", false, &[]).await;

    let patch_response = app
        .patch_json(
            &format!("/api/admin/users/{}", user.user_id),
            &admin.access_token,
            serde_json::json!({
                "username": "UpdatedUser",
                "email": "updated@example.test",
                "avatarUrl": "https://cdn.test/avatar.png"
            }),
        )
        .await;
    assert!(patch_response.status().is_success());
    let patched_user: serde_json::Value = patch_response.json().await.expect("patched user json");

    assert_eq!(patched_user["username"], "UpdatedUser");
    assert_eq!(patched_user["email"], "updated@example.test");
    assert_eq!(patched_user["avatarUrl"], "https://cdn.test/avatar.png");

    let get_response = app
        .get_json(&format!("/api/admin/users/{}", user.user_id), &admin.access_token)
        .await;
    assert!(get_response.status().is_success());
    let fetched_user: serde_json::Value = get_response.json().await.expect("fetched user json");

    assert_eq!(fetched_user["username"], "UpdatedUser");
    assert_eq!(fetched_user["email"], "updated@example.test");
    assert_eq!(fetched_user["avatarUrl"], "https://cdn.test/avatar.png");
    assert_eq!(app.audit_log_count("admin.user.updated").await, 1);
}

#[tokio::test]
#[serial]
async fn user_can_decline_invite_and_it_disappears_from_inbox() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderDecline", false, &[]).await;
    let invited = app.issue_user_token("InviteDeclineUser", false, &[]).await;

    let squad = app.create_squad(&leader, "Hotel Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;

    let invites_before = app.my_invites(&invited).await;
    assert_eq!(invites_before.len(), 1);
    assert_eq!(invites_before[0]["id"], invite_id);

    let decline_response = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/decline"),
            &invited.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(decline_response.status().is_success());

    let invites_after = app.my_invites(&invited).await;
    assert!(invites_after.is_empty());
    assert_eq!(app.user_squad_id(&invited.user_id).await, None);
    assert_eq!(app.invite_count_for_squad(squad_id).await, 0);
    assert_eq!(app.audit_log_count("user.squad.invite_declined").await, 1);
}

#[tokio::test]
#[serial]
async fn leader_can_revoke_invite_before_it_is_accepted() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderRevoke", false, &[]).await;
    let invited = app.issue_user_token("InviteRevokeUser", false, &[]).await;

    let squad = app.create_squad(&leader, "India Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;

    let revoke_response = app
        .delete(
            &format!("/api/squad-invites/{invite_id}"),
            &leader.access_token,
        )
        .await;
    assert!(revoke_response.status().is_success());

    let invites_after = app.my_invites(&invited).await;
    assert!(invites_after.is_empty());

    let accept_after_revoke = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &invited.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(accept_after_revoke.status(), reqwest::StatusCode::NOT_FOUND);
    assert_eq!(app.invite_count_for_squad(squad_id).await, 0);
    assert_eq!(app.audit_log_count("user.squad.invite_revoked").await, 1);
}

#[tokio::test]
#[serial]
async fn squad_capacity_counts_members_and_active_invites() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderCapacity", false, &[]).await;
    let member_one = app.issue_user_token("CapacityOne", false, &[]).await;
    let member_two = app.issue_user_token("CapacityTwo", false, &[]).await;
    let member_three = app.issue_user_token("CapacityThree", false, &[]).await;
    let member_four = app.issue_user_token("CapacityFour", false, &[]).await;

    let squad = app.create_squad(&leader, "Juliet Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();

    let invite_one = app.issue_invite(&leader, &squad_id, &member_one.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_one}/accept"),
        &member_one.access_token,
        serde_json::json!({}),
    )
    .await;

    let invite_two = app.issue_invite(&leader, &squad_id, &member_two.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_two}/accept"),
        &member_two.access_token,
        serde_json::json!({}),
    )
    .await;

    let invite_three = app.issue_invite(&leader, &squad_id, &member_three.user_id).await;
    let blocked_invite = app
        .post_json(
            &format!("/api/squads/{squad_id}/invites"),
            &leader.access_token,
            serde_json::json!({ "userId": member_four.user_id }),
        )
        .await;

    assert_eq!(blocked_invite.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(app.invite_count_for_squad(&squad_id).await, 1);

    let accept_third = app
        .post_json(
            &format!("/api/squad-invites/{invite_three}/accept"),
            &member_three.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(accept_third.status().is_success());
    assert_eq!(app.user_squad_id(&member_three.user_id).await.as_deref(), Some(squad_id.as_str()));
}

#[tokio::test]
#[serial]
async fn restricted_squad_member_can_leave_and_leader_can_delete() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("RestrictedAdmin", true, &[]).await;
    let leader = app.issue_user_token("RestrictedLeader", false, &[]).await;
    let member = app.issue_user_token("RestrictedMember", false, &[]).await;

    let squad = app.create_squad(&leader, "Kilo Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();
    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_id}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let restrict_response = app
        .post_json(
            &format!("/api/admin/squads/{squad_id}/restrict"),
            &admin.access_token,
            serde_json::json!({ "reason": "Temporarily hidden" }),
        )
        .await;
    assert!(restrict_response.status().is_success());

    let member_leave = app
        .post_json(
            &format!("/api/squads/{squad_id}/leave"),
            &member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(member_leave.status().is_success());
    assert_eq!(app.user_squad_id(&member.user_id).await, None);

    let delete_response = app.delete(&format!("/api/squads/{squad_id}"), &leader.access_token).await;
    assert!(delete_response.status().is_success());
    assert!(!app.squad_exists(&squad_id).await);
}

#[tokio::test]
#[serial]
async fn admin_can_deactivate_activate_and_toggle_superuser() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LifecycleAdmin", true, &[]).await;
    let user = app.issue_user_token("LifecycleUser", false, &[]).await;

    let deactivate_response = app
        .post_json(
            &format!("/api/admin/users/{}/deactivate", user.user_id),
            &admin.access_token,
            serde_json::json!({ "reason": "Violation review" }),
        )
        .await;
    assert!(deactivate_response.status().is_success());
    let deactivated: serde_json::Value =
        deactivate_response.json().await.expect("deactivated user json");
    assert_eq!(deactivated["isActive"], false);
    assert_eq!(deactivated["deactivationReason"], "Violation review");

    let me_while_deactivated = app.get_json("/api/user/me", &user.access_token).await;
    assert_eq!(me_while_deactivated.status(), reqwest::StatusCode::FORBIDDEN);

    let activate_response = app
        .post_json(
            &format!("/api/admin/users/{}/activate", user.user_id),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(activate_response.status().is_success());
    let activated: serde_json::Value = activate_response.json().await.expect("activated user json");
    assert_eq!(activated["isActive"], true);
    assert_eq!(activated["deactivationReason"], serde_json::Value::Null);

    let grant_superuser = app
        .post_json(
            &format!("/api/admin/users/{}/grant-superuser", user.user_id),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(grant_superuser.status().is_success());
    let granted: serde_json::Value = grant_superuser.json().await.expect("grant superuser json");
    assert_eq!(granted["isSuperuser"], true);

    let revoke_superuser = app
        .post_json(
            &format!("/api/admin/users/{}/revoke-superuser", user.user_id),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(revoke_superuser.status().is_success());
    let revoked: serde_json::Value = revoke_superuser.json().await.expect("revoke superuser json");
    assert_eq!(revoked["isSuperuser"], false);

    let me_after_activate = app.get_json("/api/user/me", &user.access_token).await;
    assert!(me_after_activate.status().is_success());
    assert_eq!(app.audit_log_count("admin.user.deactivated").await, 1);
    assert_eq!(app.audit_log_count("admin.user.activated").await, 1);
    assert_eq!(app.audit_log_count("admin.user.superuser_granted").await, 1);
    assert_eq!(app.audit_log_count("admin.user.superuser_revoked").await, 1);
}

#[tokio::test]
#[serial]
async fn meta_endpoints_expose_restrictions_and_squad_limits() {
    let app = TestApp::spawn().await;

    let restrictions_response = app.get_without_auth("/api/meta/restrictions").await;
    assert!(restrictions_response.status().is_success());
    let restrictions: serde_json::Value =
        restrictions_response.json().await.expect("restrictions meta json");
    assert!(restrictions.as_array().expect("restrictions array").len() >= 2);
    assert!(restrictions
        .as_array()
        .expect("restrictions array")
        .iter()
        .any(|item| item["key"] == "create_squad" && item["locale"]["en"]["title"].is_string()));

    let squad_config_response = app.get_without_auth("/api/meta/squads/config").await;
    assert!(squad_config_response.status().is_success());
    let squad_config: serde_json::Value =
        squad_config_response.json().await.expect("squad config json");
    assert_eq!(squad_config["maxMembers"], 4);
    assert_eq!(squad_config["inviteTtlHours"], 24);
    assert_eq!(squad_config["nameMinChars"], 4);
    assert_eq!(squad_config["nameMaxChars"], 16);
}

#[tokio::test]
#[serial]
async fn stranger_cannot_accept_or_decline_someone_elses_invite() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderOwnership", false, &[]).await;
    let invited = app.issue_user_token("OwnedInviteUser", false, &[]).await;
    let stranger = app.issue_user_token("InviteStranger", false, &[]).await;

    let squad = app.create_squad(&leader, "Lima Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;

    let accept_as_stranger = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &stranger.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(accept_as_stranger.status(), reqwest::StatusCode::FORBIDDEN);

    let decline_as_stranger = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/decline"),
            &stranger.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(decline_as_stranger.status(), reqwest::StatusCode::FORBIDDEN);

    let invites_for_real_user = app.my_invites(&invited).await;
    assert_eq!(invites_for_real_user.len(), 1);
    assert_eq!(app.user_squad_id(&stranger.user_id).await, None);
}

#[tokio::test]
#[serial]
async fn non_leader_cannot_invite_kick_or_patch_squad() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderRights", false, &[]).await;
    let member = app.issue_user_token("MemberRights", false, &[]).await;
    let outsider = app.issue_user_token("OutsiderRights", false, &[]).await;
    let target = app.issue_user_token("TargetRights", false, &[]).await;

    let squad = app.create_squad(&leader, "Mike Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();
    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_id}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let invite_as_member = app
        .post_json(
            &format!("/api/squads/{squad_id}/invites"),
            &member.access_token,
            serde_json::json!({ "userId": target.user_id }),
        )
        .await;
    assert_eq!(invite_as_member.status(), reqwest::StatusCode::FORBIDDEN);

    let kick_as_member = app
        .post_json(
            &format!("/api/squads/{squad_id}/members/{}/kick", leader.user_id),
            &member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(kick_as_member.status(), reqwest::StatusCode::FORBIDDEN);

    let patch_as_outsider = app
        .patch_json(
            &format!("/api/squads/{squad_id}"),
            &outsider.access_token,
            serde_json::json!({ "name": "Renamed Squad" }),
        )
        .await;
    assert_eq!(patch_as_outsider.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn admin_cannot_kick_squad_leader() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LeaderKickAdmin", true, &[]).await;
    let leader = app.issue_user_token("LeaderProtected", false, &[]).await;

    let squad = app.create_squad(&leader, "November Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();

    let kick_leader = app
        .post_json(
            &format!("/api/admin/squads/{squad_id}/members/{}/kick", leader.user_id),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(kick_leader.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(app.user_squad_id(&leader.user_id).await.as_deref(), Some(squad_id.as_str()));
    assert_eq!(app.audit_log_count("admin.squad.member_kicked").await, 0);
}

#[tokio::test]
#[serial]
async fn user_cannot_create_squad_when_already_in_one() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderExisting", false, &[]).await;
    let member = app.issue_user_token("MemberExisting", false, &[]).await;

    let squad = app.create_squad(&leader, "Oscar Team").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();
    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{invite_id}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let second_squad_attempt = app
        .post_json(
            "/api/squads",
            &member.access_token,
            serde_json::json!({ "name": "Should Fail" }),
        )
        .await;
    assert_eq!(second_squad_attempt.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn user_cannot_accept_invite_when_already_in_squad() {
    let app = TestApp::spawn().await;
    let leader_one = app.issue_user_token("LeaderOneInvite", false, &[]).await;
    let leader_two = app.issue_user_token("LeaderTwoInvite", false, &[]).await;
    let member = app.issue_user_token("MemberBusy", false, &[]).await;

    let first_squad = app.create_squad(&leader_one, "Papa Team").await;
    let first_squad_id = first_squad["id"].as_str().expect("first squad id").to_string();
    let first_invite = app.issue_invite(&leader_one, &first_squad_id, &member.user_id).await;
    app.post_json(
        &format!("/api/squad-invites/{first_invite}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let second_squad = app.create_squad(&leader_two, "Quebec Team").await;
    let second_squad_id = second_squad["id"].as_str().expect("second squad id").to_string();
    let second_invite_attempt = app
        .post_json(
            &format!("/api/squads/{second_squad_id}/invites"),
            &leader_two.access_token,
            serde_json::json!({ "userId": member.user_id }),
        )
        .await;
    assert_eq!(second_invite_attempt.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(app.user_squad_id(&member.user_id).await.as_deref(), Some(first_squad_id.as_str()));
    assert_eq!(app.invite_count_for_squad(&second_squad_id).await, 0);
}

#[tokio::test]
#[serial]
async fn non_superuser_cannot_access_admin_endpoints() {
    let app = TestApp::spawn().await;
    let user = app.issue_user_token("PlainUser", false, &[]).await;

    let admin_me = app.get_json("/api/admin/me", &user.access_token).await;
    assert_eq!(admin_me.status(), reqwest::StatusCode::FORBIDDEN);

    let admin_users = app
        .get_json("/api/admin/users?page=1&perPage=10", &user.access_token)
        .await;
    assert_eq!(admin_users.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn debug_issue_token_endpoint_requires_debug_only_route_and_no_auth() {
    let app = TestApp::spawn().await;

    let response = app
        .post_without_auth(
            "/api/test/issue-token",
            serde_json::json!({
                "username": "RouteProbe",
                "isSuperuser": false,
                "restrictions": []
            }),
        )
        .await;

    assert!(response.status().is_success());
}
