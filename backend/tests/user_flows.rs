mod common;

use common::TestApp;
use image::{ImageBuffer, Rgba};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;

fn make_skin_png(fill: [u8; 4]) -> Vec<u8> {
    let image = ImageBuffer::from_pixel(64, 64, Rgba(fill));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("write png");
    bytes
}

#[tokio::test]
#[serial]
async fn startup_seeds_required_system_assets() {
    let app = TestApp::spawn().await;

    let subscription_plus = app
        .asset_by_key("subscription_plus")
        .await
        .expect("subscription_plus asset");
    assert_eq!(subscription_plus.asset_kind, "subscription");
    assert_eq!(subscription_plus.ownership_model, "expirable");
    assert!(!subscription_plus.is_currency);

    let subscription_pro = app
        .asset_by_key("subscription_pro")
        .await
        .expect("subscription_pro asset");
    assert_eq!(subscription_pro.asset_kind, "subscription");
    assert_eq!(subscription_pro.ownership_model, "expirable");
    assert!(!subscription_pro.is_currency);

    let coin_default = app
        .asset_by_key("coin_default")
        .await
        .expect("coin_default asset");
    assert_eq!(coin_default.asset_kind, "currency");
    assert_eq!(coin_default.ownership_model, "stackable");
    assert!(coin_default.is_currency);
}

#[tokio::test]
#[serial]
async fn seeded_system_assets_keep_user_edited_display_fields() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("AssetAdmin", true, &[]).await;

    let asset = app
        .asset_by_key("subscription_plus")
        .await
        .expect("subscription_plus asset");

    let patched = app
        .patch_asset(
            &admin,
            &asset.id.to_string(),
            serde_json::json!({
                "display_name": "Premium Plus",
                "description": "Custom renamed tier"
            }),
        )
        .await;

    assert_eq!(patched["displayName"], "Premium Plus");
    assert_eq!(patched["description"], "Custom renamed tier");

    auth::services::ownership::catalog::ensure_system_assets(&app.db)
        .await
        .expect("rerun system asset seeding");

    let updated = app
        .asset_by_key("subscription_plus")
        .await
        .expect("subscription_plus asset after reseed");
    assert_eq!(updated.display_name, "Premium Plus");
    assert_eq!(updated.description.as_deref(), Some("Custom renamed tier"));
}

#[tokio::test]
#[serial]
async fn registration_requires_legal_acceptance_before_user_is_created() {
    let app = TestApp::spawn().await;
    let registration_token = uuid::Uuid::new_v4().to_string();

    auth::entities::AuthRayActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        pow_prefix: Set("pending-legal-pref".to_string()),
        pow_complexity: Set(1),
        delivery_method: Set(auth::entities::AuthRayTokenDeliveryMethod::Redirect),
        delivery_target: Set("/profile".to_string()),
        registration_token: Set(Some(registration_token.clone())),
        pending_discord_id: Set(Some("discord-legal-block".to_string())),
        pending_username: Set(Some("LegalBlockUser".to_string())),
        pending_avatar_url: Set(Some("https://cdn.discord.test/legal.png".to_string())),
        pending_email: Set(Some("legal-block@example.com".to_string())),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(&app.db)
    .await
    .expect("insert pending auth ray");

    let response = app
        .post_without_auth(
            "/api/auth/register",
            serde_json::json!({
                "registrationToken": registration_token,
                "acceptedUserAgreement": false,
                "acceptedPrivacyPolicy": true
            }),
        )
        .await;

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await.expect("register error json");
    assert_eq!(
        body["error"],
        "User agreement must be accepted before registration"
    );

    let user = auth::entities::User::find()
        .filter(auth::entities::UserColumn::DiscordId.eq("discord-legal-block"))
        .one(&app.db)
        .await
        .expect("lookup user");
    assert!(user.is_none());
}

#[tokio::test]
#[serial]
async fn registration_creates_user_and_audits_legal_acceptance() {
    let app = TestApp::spawn().await;
    let registration_token = uuid::Uuid::new_v4().to_string();

    auth::entities::AuthRayActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        pow_prefix: Set("pending-legal-ok".to_string()),
        pow_complexity: Set(1),
        delivery_method: Set(auth::entities::AuthRayTokenDeliveryMethod::Redirect),
        delivery_target: Set("/profile".to_string()),
        registration_token: Set(Some(registration_token.clone())),
        pending_discord_id: Set(Some("discord-legal-ok".to_string())),
        pending_username: Set(Some("LegalAgreeUser".to_string())),
        pending_avatar_url: Set(Some("https://cdn.discord.test/ok.png".to_string())),
        pending_email: Set(Some("legal-ok@example.com".to_string())),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(&app.db)
    .await
    .expect("insert pending auth ray");

    let response = app
        .post_without_auth(
            "/api/auth/register",
            serde_json::json!({
                "registrationToken": registration_token,
                "acceptedUserAgreement": true,
                "acceptedPrivacyPolicy": true
            }),
        )
        .await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = response.json().await.expect("register json");
    assert_eq!(body["status"], "authorized");
    assert!(body["accessToken"].as_str().is_some());
    assert_eq!(body["username"], "LegalAgreeUser");

    let user = auth::entities::User::find()
        .filter(auth::entities::UserColumn::DiscordId.eq("discord-legal-ok"))
        .one(&app.db)
        .await
        .expect("lookup created user")
        .expect("created user exists");
    assert_eq!(user.username, "LegalAgreeUser");

    let pending_ray = auth::entities::AuthRay::find()
        .filter(auth::entities::AuthRayColumn::RegistrationToken.eq(registration_token))
        .one(&app.db)
        .await
        .expect("lookup pending auth ray");
    assert!(pending_ray.is_none());

    assert_eq!(
        app.audit_log_count(auth::services::audit::ACTION_USER_REGISTERED)
            .await,
        1
    );
    assert_eq!(
        app.audit_log_count(auth::services::audit::ACTION_USER_LEGAL_ACCEPTED)
            .await,
        1
    );
}

#[tokio::test]
#[serial]
async fn system_asset_seeding_fails_when_existing_asset_breaks_invariants() {
    let app = TestApp::spawn().await;

    let asset = app
        .asset_by_key("coin_default")
        .await
        .expect("coin_default asset");
    let mut active: auth::entities::AssetDefinitionActiveModel = asset.into();
    active.is_currency = Set(false);
    active
        .update(&app.db)
        .await
        .expect("break coin_default invariant");

    let error = auth::services::ownership::catalog::ensure_system_assets(&app.db)
        .await
        .expect_err("expected invariant validation error");
    assert!(error.to_string().contains("coin_default"));
    assert!(error.to_string().contains("is_currency"));
}

#[tokio::test]
#[serial]
async fn user_can_create_squad_and_see_it_in_profile() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderOne", false, &[]).await;

    let squad = app.create_squad(&leader, "Alpha Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");

    let my_squad_response = app
        .get_json("/api/user/me/squad", &leader.access_token)
        .await;
    assert!(my_squad_response.status().is_success());
    let my_squad: serde_json::Value = my_squad_response.json().await.expect("my squad json");

    assert_eq!(my_squad["id"], squad_id);
    assert_eq!(my_squad["memberCount"], 1);
    assert_eq!(
        app.user_squad_id(&leader.user_id).await.as_deref(),
        Some(squad_id)
    );
    assert_eq!(app.audit_log_count("user.squad.created").await, 1);
}

#[tokio::test]
#[serial]
async fn admin_squad_search_is_case_insensitive() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("CaseSquadAdmin", true, &[]).await;
    let leader = app.issue_user_token("CaseSquadLeader", false, &[]).await;
    let squad = app.create_squad(&leader, "MixedCaseSquad").await;
    let squad_id = squad["id"].as_str().expect("squad id");

    let response = app
        .get_json(
            "/api/admin/squads?q=mixedcasesquad&page=1&perPage=10",
            &admin.access_token,
        )
        .await;
    assert!(response.status().is_success());

    let body: serde_json::Value = response
        .json()
        .await
        .expect("case-insensitive admin squad search");
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["id"], squad_id);
    assert_eq!(body["items"][0]["name"], "MixedCaseSquad");
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
    assert_eq!(
        app.user_squad_id(&invited.user_id).await.as_deref(),
        Some(squad_id)
    );
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
async fn squad_image_can_be_uploaded_fetched_and_deleted() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderImage", false, &[]).await;
    let squad = app.create_squad(&leader, "Image Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let png = make_skin_png([64, 128, 255, 255]);

    let upload = app
        .post_multipart(
            &format!("/api/squads/{squad_id}/image"),
            &leader.access_token,
            "squad.png",
            "image/png",
            png.clone(),
        )
        .await;
    assert!(upload.status().is_success());

    let get_image = app
        .get_bytes_without_auth(&format!("/api/squads/{squad_id}/image"))
        .await;
    assert!(get_image.status().is_success());
    assert_eq!(
        get_image
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
    let fetched = get_image.bytes().await.expect("image bytes");
    assert!(!fetched.is_empty());

    let delete = app
        .delete(
            &format!("/api/squads/{squad_id}/image"),
            &leader.access_token,
        )
        .await;
    assert!(delete.status().is_success());

    let missing = app
        .get_bytes_without_auth(&format!("/api/squads/{squad_id}/image"))
        .await;
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);
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

    let delete_response = app
        .delete(&format!("/api/squads/{squad_id}"), &leader.access_token)
        .await;
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
            &format!(
                "/api/admin/squads/{squad_id}/members/{}/kick",
                member.user_id
            ),
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
    assert_eq!(
        app.audit_log_count("admin.user.restriction_granted").await,
        1
    );
    assert_eq!(
        app.audit_log_count("admin.user.restriction_revoked").await,
        1
    );
}

#[tokio::test]
#[serial]
async fn admin_can_search_users_by_username_and_uuid() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SearchAdmin", true, &[]).await;
    let searched_user = app.issue_user_token("UniqueSearchTarget", false, &[]).await;
    let _another_user = app
        .issue_user_token("AnotherSearchTarget", false, &[])
        .await;

    let username_search = app
        .get_json(
            "/api/admin/users?q=UniqueSearchTarget&page=1&perPage=10",
            &admin.access_token,
        )
        .await;
    assert!(username_search.status().is_success());
    let username_search_body: serde_json::Value =
        username_search.json().await.expect("username search json");

    assert_eq!(username_search_body["total"], 1);
    assert_eq!(
        username_search_body["items"][0]["id"],
        searched_user.user_id
    );
    assert_eq!(
        username_search_body["items"][0]["username"],
        "UniqueSearchTarget"
    );

    let uuid_search = app
        .get_json(
            &format!(
                "/api/admin/users?q={}&page=1&perPage=10",
                searched_user.user_id
            ),
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
async fn admin_user_search_is_case_insensitive() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("CaseSearchAdmin", true, &[]).await;
    let searched_user = app.issue_user_token("MixedCaseTarget", false, &[]).await;

    let response = app
        .get_json(
            "/api/admin/users?q=mixedcasetarget&page=1&perPage=10",
            &admin.access_token,
        )
        .await;
    assert!(response.status().is_success());

    let body: serde_json::Value = response
        .json()
        .await
        .expect("case-insensitive admin user search");
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["id"], searched_user.user_id);
    assert_eq!(body["items"][0]["username"], "MixedCaseTarget");
}

#[tokio::test]
#[serial]
async fn authenticated_user_can_search_users_by_username_for_autocomplete() {
    let app = TestApp::spawn().await;
    let requester = app.issue_user_token("SearchRequester", false, &[]).await;
    let alpha = app.issue_user_token("AlphaSearchOne", false, &[]).await;
    let beta = app.issue_user_token("AlphaSearchTwo", false, &[]).await;
    let _gamma = app.issue_user_token("AlphaSearchThree", false, &[]).await;
    let _other = app.issue_user_token("DifferentPlayer", false, &[]).await;

    let response = app
        .get_json(
            "/api/users/search?q=AlphaSearch&limit=3",
            &requester.access_token,
        )
        .await;
    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("user search json");
    let items = body.as_array().expect("user search array");

    assert_eq!(items.len(), 3);
    assert!(items.iter().any(|item| item["id"] == alpha.user_id));
    assert!(items.iter().any(|item| item["id"] == beta.user_id));
    assert!(items.iter().all(|item| item.get("discordId").is_none()));
    assert!(
        items
            .iter()
            .all(|item| item["username"].as_str().unwrap().contains("AlphaSearch"))
    );
}

#[tokio::test]
#[serial]
async fn authenticated_user_search_is_case_insensitive() {
    let app = TestApp::spawn().await;
    let requester = app
        .issue_user_token("CaseInsensitiveRequester", false, &[])
        .await;
    let searched_user = app.issue_user_token("CaseFoldPlayer", false, &[]).await;

    let response = app
        .get_json(
            "/api/users/search?q=casefoldplayer",
            &requester.access_token,
        )
        .await;
    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("case-insensitive user search");
    let items = body.as_array().expect("case-insensitive user array");
    assert!(items.iter().any(|item| item["id"] == searched_user.user_id));
    assert!(
        items
            .iter()
            .any(|item| item["username"] == "CaseFoldPlayer")
    );
}

#[tokio::test]
#[serial]
async fn user_search_requires_minimum_query_length_and_authentication() {
    let app = TestApp::spawn().await;
    let requester = app.issue_user_token("MinSearchRequester", false, &[]).await;

    let too_short = app
        .get_json("/api/users/search?q=ab", &requester.access_token)
        .await;
    assert_eq!(too_short.status().as_u16(), 400);

    let unauthorized = app.get_without_auth("/api/users/search?q=Alpha").await;
    assert_eq!(unauthorized.status().as_u16(), 401);
}

#[tokio::test]
#[serial]
async fn user_search_uses_default_limit_of_ten_results() {
    let app = TestApp::spawn().await;
    let requester = app
        .issue_user_token("DefaultLimitRequester", false, &[])
        .await;

    for index in 0..12 {
        let username = format!("GroupSearch{:02}", index);
        let _ = app.issue_user_token(&username, false, &[]).await;
    }

    let response = app
        .get_json("/api/users/search?q=GroupSearch", &requester.access_token)
        .await;
    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("default limit json");
    let items = body.as_array().expect("default limit array");
    assert_eq!(items.len(), 10);
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
        .get_json(
            &format!("/api/admin/users/{}", user.user_id),
            &admin.access_token,
        )
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
async fn admin_can_get_user_squad_rename_squad_and_delete_user_skin() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("AdminSquadTools", true, &[]).await;
    let leader = app.issue_user_token("SkinLeader", false, &[]).await;
    let member = app.issue_user_token("SkinMember", false, &[]).await;

    let squad = app.create_squad(&leader, "Original Squad").await;
    let squad_id = squad["id"].as_str().expect("squad id").to_string();
    let invite_id = app.issue_invite(&leader, &squad_id, &member.user_id).await;
    let accept = app
        .post_json(
            &format!("/api/squad-invites/{invite_id}/accept"),
            &member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(accept.status().is_success());

    let get_user_squad = app
        .get_json(
            &format!("/api/admin/user/{}/squad", member.user_id),
            &admin.access_token,
        )
        .await;
    assert!(get_user_squad.status().is_success());
    let user_squad: serde_json::Value = get_user_squad.json().await.expect("admin user squad json");
    assert_eq!(user_squad["id"], squad_id);
    assert_eq!(user_squad["name"], "Original Squad");
    assert_eq!(user_squad["memberCount"], 2);

    let renamed = app
        .patch_json(
            &format!("/api/admin/squads/{squad_id}"),
            &admin.access_token,
            serde_json::json!({ "name": "Renamed By Admin" }),
        )
        .await;
    assert!(renamed.status().is_success());
    let renamed_body: serde_json::Value = renamed.json().await.expect("renamed squad json");
    assert_eq!(renamed_body["name"], "Renamed By Admin");

    let png = make_skin_png([32, 180, 220, 255]);
    let upload_skin = app
        .post_multipart(
            "/api/skins/me?model=default",
            &member.access_token,
            "member-skin.png",
            "image/png",
            png,
        )
        .await;
    assert!(upload_skin.status().is_success());

    let get_skin_before = app
        .get_bytes_without_auth(&format!("/api/skins/{}", member.user_id))
        .await;
    assert!(get_skin_before.status().is_success());

    let delete_skin = app
        .delete(
            &format!("/api/admin/user/{}/skin", member.user_id),
            &admin.access_token,
        )
        .await;
    assert!(delete_skin.status().is_success());

    let get_skin_after = app
        .get_bytes_without_auth(&format!("/api/skins/{}", member.user_id))
        .await;
    assert_eq!(get_skin_after.status(), reqwest::StatusCode::NOT_FOUND);

    assert_eq!(app.audit_log_count("admin.squad.updated").await, 1);
    assert_eq!(app.audit_log_count("admin.user.skin.deleted").await, 1);
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
    assert_eq!(invites_before[0]["squadName"], "Hotel Team");
    assert_eq!(invites_before[0]["inviterUserId"], leader.user_id);
    assert_eq!(invites_before[0]["inviterUsername"], "LeaderDecline");
    assert_eq!(invites_before[0]["invitedUserId"], invited.user_id);
    assert_eq!(invites_before[0]["invitedUsername"], "InviteDeclineUser");
    assert!(invites_before[0]["inviterAvatarUrl"].is_null());

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
async fn squad_members_endpoint_includes_active_pending_invites() {
    let app = TestApp::spawn().await;
    let leader = app.issue_user_token("LeaderRoster", false, &[]).await;
    let invited = app.issue_user_token("PendingRoster", false, &[]).await;

    let squad = app.create_squad(&leader, "India Team").await;
    let squad_id = squad["id"].as_str().expect("squad id");
    let invite_id = app.issue_invite(&leader, squad_id, &invited.user_id).await;

    let response = app
        .get_json(&format!("/api/squads/{squad_id}/members"), "")
        .await;
    assert!(response.status().is_success());
    let members: Vec<serde_json::Value> = response.json().await.expect("members json");

    assert_eq!(members.len(), 2);
    assert_eq!(members[0]["id"], leader.user_id);
    assert_eq!(members[0]["username"], "LeaderRoster");
    assert_eq!(members[0]["isLeader"], true);
    assert_eq!(members[0]["isPendingInvite"], false);
    assert!(members[0]["inviteId"].is_null());

    assert_eq!(members[1]["id"], invited.user_id);
    assert_eq!(members[1]["username"], "PendingRoster");
    assert_eq!(members[1]["isLeader"], false);
    assert_eq!(members[1]["isPendingInvite"], true);
    assert_eq!(members[1]["inviteId"], invite_id);
}

#[tokio::test]
#[serial]
async fn service_endpoint_returns_squads_that_contain_any_of_requested_users() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SquadLookupAdmin", true, &[]).await;
    let leader_alpha = app.issue_user_token("LeaderAlpha", false, &[]).await;
    let leader_beta = app.issue_user_token("LeaderBeta", false, &[]).await;
    let alpha_member = app.issue_user_token("AlphaMember", false, &[]).await;
    let beta_member = app.issue_user_token("BetaMember", false, &[]).await;
    let outsider = app.issue_user_token("NoSquadUser", false, &[]).await;

    let alpha_squad = app.create_squad(&leader_alpha, "Alpha Team").await;
    let alpha_squad_id = alpha_squad["id"]
        .as_str()
        .expect("alpha squad id")
        .to_string();
    let alpha_invite = app
        .issue_invite(&leader_alpha, &alpha_squad_id, &alpha_member.user_id)
        .await;
    let alpha_accept = app
        .post_json(
            &format!("/api/squad-invites/{alpha_invite}/accept"),
            &alpha_member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(alpha_accept.status().is_success());

    let beta_squad = app.create_squad(&leader_beta, "Beta Team").await;
    let beta_squad_id = beta_squad["id"]
        .as_str()
        .expect("beta squad id")
        .to_string();
    let beta_invite = app
        .issue_invite(&leader_beta, &beta_squad_id, &beta_member.user_id)
        .await;
    let beta_accept = app
        .post_json(
            &format!("/api/squad-invites/{beta_invite}/accept"),
            &beta_member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(beta_accept.status().is_success());

    let created_token = app
        .post_json(
            "/api/admin/service-tokens",
            &admin.access_token,
            serde_json::json!({ "systemName": "squad_lookup" }),
        )
        .await;
    assert!(created_token.status().is_success());
    let created_token_body: serde_json::Value =
        created_token.json().await.expect("service token json");
    let service_token = created_token_body["plaintextToken"]
        .as_str()
        .expect("plaintext token")
        .to_string();

    let response = app
        .post_json(
            "/api/service/squads/by-users",
            &service_token,
            serde_json::json!({
                "userIds": [
                    alpha_member.user_id,
                    beta_member.user_id,
                    outsider.user_id
                ]
            }),
        )
        .await;
    assert!(
        response.status().is_success(),
        "{}",
        response.text().await.unwrap_or_default()
    );
    let body: serde_json::Value = response.json().await.expect("service squad lookup json");
    let items = body.as_array().expect("service squad lookup array");
    assert_eq!(items.len(), 2);

    assert_eq!(items[0]["squad"]["name"], "Alpha Team");
    assert_eq!(items[0]["squad"]["memberCount"], 2);
    assert_eq!(
        items[0]["matchedUsers"]
            .as_array()
            .expect("alpha matched")
            .len(),
        1
    );
    assert_eq!(items[0]["matchedUsers"][0]["id"], alpha_member.user_id);
    assert_eq!(items[0]["matchedUsers"][0]["username"], "AlphaMember");

    assert_eq!(items[1]["squad"]["name"], "Beta Team");
    assert_eq!(items[1]["squad"]["memberCount"], 2);
    assert_eq!(
        items[1]["matchedUsers"]
            .as_array()
            .expect("beta matched")
            .len(),
        1
    );
    assert_eq!(items[1]["matchedUsers"][0]["id"], beta_member.user_id);
    assert_eq!(items[1]["matchedUsers"][0]["username"], "BetaMember");
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

    let invite_one = app
        .issue_invite(&leader, &squad_id, &member_one.user_id)
        .await;
    app.post_json(
        &format!("/api/squad-invites/{invite_one}/accept"),
        &member_one.access_token,
        serde_json::json!({}),
    )
    .await;

    let invite_two = app
        .issue_invite(&leader, &squad_id, &member_two.user_id)
        .await;
    app.post_json(
        &format!("/api/squad-invites/{invite_two}/accept"),
        &member_two.access_token,
        serde_json::json!({}),
    )
    .await;

    let invite_three = app
        .issue_invite(&leader, &squad_id, &member_three.user_id)
        .await;
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
    assert_eq!(
        app.user_squad_id(&member_three.user_id).await.as_deref(),
        Some(squad_id.as_str())
    );
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

    let delete_response = app
        .delete(&format!("/api/squads/{squad_id}"), &leader.access_token)
        .await;
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
    let deactivated: serde_json::Value = deactivate_response
        .json()
        .await
        .expect("deactivated user json");
    assert_eq!(deactivated["isActive"], false);
    assert_eq!(deactivated["deactivationReason"], "Violation review");

    let me_while_deactivated = app.get_json("/api/user/me", &user.access_token).await;
    assert_eq!(
        me_while_deactivated.status(),
        reqwest::StatusCode::FORBIDDEN
    );

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
    let revoked: serde_json::Value = revoke_superuser
        .json()
        .await
        .expect("revoke superuser json");
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
    let restrictions: serde_json::Value = restrictions_response
        .json()
        .await
        .expect("restrictions meta json");
    assert!(restrictions.as_array().expect("restrictions array").len() >= 2);
    assert!(
        restrictions
            .as_array()
            .expect("restrictions array")
            .iter()
            .any(|item| item["key"] == "create_squad" && item["locale"]["en"]["title"].is_string())
    );

    let squad_config_response = app.get_without_auth("/api/meta/squads/config").await;
    assert!(squad_config_response.status().is_success());
    let squad_config: serde_json::Value = squad_config_response
        .json()
        .await
        .expect("squad config json");
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
            &format!(
                "/api/admin/squads/{squad_id}/members/{}/kick",
                leader.user_id
            ),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(kick_leader.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(
        app.user_squad_id(&leader.user_id).await.as_deref(),
        Some(squad_id.as_str())
    );
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
    assert_eq!(
        second_squad_attempt.status(),
        reqwest::StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[serial]
async fn user_cannot_accept_invite_when_already_in_squad() {
    let app = TestApp::spawn().await;
    let leader_one = app.issue_user_token("LeaderOneInvite", false, &[]).await;
    let leader_two = app.issue_user_token("LeaderTwoInvite", false, &[]).await;
    let member = app.issue_user_token("MemberBusy", false, &[]).await;

    let first_squad = app.create_squad(&leader_one, "Papa Team").await;
    let first_squad_id = first_squad["id"]
        .as_str()
        .expect("first squad id")
        .to_string();
    let first_invite = app
        .issue_invite(&leader_one, &first_squad_id, &member.user_id)
        .await;
    app.post_json(
        &format!("/api/squad-invites/{first_invite}/accept"),
        &member.access_token,
        serde_json::json!({}),
    )
    .await;

    let second_squad = app.create_squad(&leader_two, "Quebec Team").await;
    let second_squad_id = second_squad["id"]
        .as_str()
        .expect("second squad id")
        .to_string();
    let second_invite_attempt = app
        .post_json(
            &format!("/api/squads/{second_squad_id}/invites"),
            &leader_two.access_token,
            serde_json::json!({ "userId": member.user_id }),
        )
        .await;
    assert_eq!(
        second_invite_attempt.status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.user_squad_id(&member.user_id).await.as_deref(),
        Some(first_squad_id.as_str())
    );
    assert_eq!(app.invite_count_for_squad(&second_squad_id).await, 0);
}

#[tokio::test]
#[serial]
async fn accepting_invite_clears_all_other_pending_invites_for_user() {
    let app = TestApp::spawn().await;
    let leader_one = app.issue_user_token("LeaderInboxOne", false, &[]).await;
    let leader_two = app.issue_user_token("LeaderInboxTwo", false, &[]).await;
    let member = app.issue_user_token("InviteInboxUser", false, &[]).await;

    let first_squad = app.create_squad(&leader_one, "Romeo Team").await;
    let first_squad_id = first_squad["id"]
        .as_str()
        .expect("first squad id")
        .to_string();
    let second_squad = app.create_squad(&leader_two, "Sierra Team").await;
    let second_squad_id = second_squad["id"]
        .as_str()
        .expect("second squad id")
        .to_string();

    let first_invite = app
        .issue_invite(&leader_one, &first_squad_id, &member.user_id)
        .await;
    let _second_invite = app
        .issue_invite(&leader_two, &second_squad_id, &member.user_id)
        .await;

    let invites_before = app.my_invites(&member).await;
    assert_eq!(invites_before.len(), 2);

    let accept_response = app
        .post_json(
            &format!("/api/squad-invites/{first_invite}/accept"),
            &member.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(accept_response.status().is_success());

    assert_eq!(
        app.user_squad_id(&member.user_id).await.as_deref(),
        Some(first_squad_id.as_str())
    );
    assert!(app.my_invites(&member).await.is_empty());
    assert_eq!(app.invite_count_for_squad(&first_squad_id).await, 0);
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

#[tokio::test]
#[serial]
async fn admin_can_manage_assets_and_public_catalog_only_shows_public_active_entries() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("AssetAdmin", true, &[]).await;

    let public_asset = app
        .create_asset(
            &admin,
            serde_json::json!({
                "key": "premium_pass",
                "display_name": "Premium Pass",
                "description": "Premium access",
                "asset_kind": "subscription",
                "ownership_model": "expirable",
                "is_currency": false,
                "is_user_purchasable": true,
                "is_public": true,
                "metadata": { "days": 30 }
            }),
        )
        .await;
    let hidden_asset = app
        .create_asset(
            &admin,
            serde_json::json!({
                "key": "admin_only_token",
                "display_name": "Admin Token",
                "description": null,
                "asset_kind": "token",
                "ownership_model": "stackable",
                "is_currency": false,
                "is_user_purchasable": false,
                "is_public": false,
                "metadata": {}
            }),
        )
        .await;

    let public_list = app.get_without_auth("/api/assets").await;
    assert!(public_list.status().is_success());
    let public_list_body: serde_json::Value = public_list.json().await.expect("public assets json");
    let public_keys: Vec<_> = public_list_body["items"]
        .as_array()
        .expect("public asset items")
        .iter()
        .map(|item| item["key"].as_str().expect("public asset key"))
        .collect();
    assert!(public_keys.contains(&"premium_pass"));
    assert!(public_keys.contains(&"subscription_plus"));
    assert!(public_keys.contains(&"subscription_pro"));
    assert!(public_keys.contains(&"coin_default"));
    assert_eq!(public_list_body["total"], 4);

    let admin_list = app.get_json("/api/admin/assets", &admin.access_token).await;
    assert!(admin_list.status().is_success());
    let admin_list_body: serde_json::Value = admin_list.json().await.expect("admin assets json");
    assert_eq!(admin_list_body["total"], 5);
    let admin_keys: Vec<_> = admin_list_body["items"]
        .as_array()
        .expect("admin asset items")
        .iter()
        .map(|item| item["key"].as_str().expect("admin asset key"))
        .collect();
    assert!(admin_keys.contains(&"admin_only_token"));

    let patch_response = app
        .patch_json(
            &format!(
                "/api/admin/assets/{}",
                hidden_asset["id"].as_str().expect("asset id")
            ),
            &admin.access_token,
            serde_json::json!({
                "display_name": "Admin Token Updated",
                "is_public": true,
                "is_active": false
            }),
        )
        .await;
    assert!(patch_response.status().is_success());

    let public_get = app
        .get_without_auth(&format!(
            "/api/assets/{}",
            public_asset["id"].as_str().expect("asset id")
        ))
        .await;
    assert!(public_get.status().is_success());

    let hidden_public_get = app
        .get_without_auth(&format!(
            "/api/assets/{}",
            hidden_asset["id"].as_str().expect("asset id")
        ))
        .await;
    assert_eq!(hidden_public_get.status(), reqwest::StatusCode::NOT_FOUND);

    assert_eq!(app.audit_log_count("admin.asset.created").await, 2);
    assert_eq!(app.audit_log_count("admin.asset.updated").await, 1);
}

#[tokio::test]
#[serial]
async fn asset_keys_must_be_lowercase_and_url_safe() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("KeyAdmin", true, &[]).await;

    let uppercase = app
        .post_json(
            "/api/admin/assets",
            &admin.access_token,
            serde_json::json!({
                "key": "BadKey",
                "display_name": "Bad",
                "description": null,
                "asset_kind": "item",
                "ownership_model": "entitlement",
                "is_currency": false,
                "is_user_purchasable": false,
                "is_public": false,
                "metadata": {}
            }),
        )
        .await;
    assert_eq!(uppercase.status(), reqwest::StatusCode::BAD_REQUEST);

    let invalid_char = app
        .post_json(
            "/api/admin/assets",
            &admin.access_token,
            serde_json::json!({
                "key": "bad/key",
                "display_name": "Bad",
                "description": null,
                "asset_kind": "item",
                "ownership_model": "entitlement",
                "is_currency": false,
                "is_user_purchasable": false,
                "is_public": false,
                "metadata": {}
            }),
        )
        .await;
    assert_eq!(invalid_char.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn kit_assets_must_be_stackable() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("KitAssetAdmin", true, &[]).await;

    let kit = app
        .post_json(
            "/api/admin/assets",
            &admin.access_token,
            serde_json::json!({
                "key": "starter_kit",
                "display_name": "Starter Kit",
                "description": null,
                "asset_kind": "kit",
                "ownership_model": "stackable",
                "is_currency": false,
                "is_user_purchasable": true,
                "is_public": true,
                "metadata": {}
            }),
        )
        .await;
    assert!(
        kit.status().is_success(),
        "{}",
        kit.text().await.unwrap_or_default()
    );

    let invalid_kit = app
        .post_json(
            "/api/admin/assets",
            &admin.access_token,
            serde_json::json!({
                "key": "bad_kit",
                "display_name": "Bad Kit",
                "description": null,
                "asset_kind": "kit",
                "ownership_model": "entitlement",
                "is_currency": false,
                "is_user_purchasable": false,
                "is_public": false,
                "metadata": {}
            }),
        )
        .await;
    assert_eq!(invalid_kit.status(), reqwest::StatusCode::BAD_REQUEST);
    let invalid_body: serde_json::Value = invalid_kit.json().await.expect("invalid kit body");
    assert_eq!(
        invalid_body["problem"]["detail"],
        "Kit assets must use stackable ownership"
    );
}

#[tokio::test]
#[serial]
async fn admin_can_grant_and_revoke_entitlement_idempotently_and_user_sees_it() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("EntitlementAdmin", true, &[]).await;
    let user = app.issue_user_token("EntitlementUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "dragon_skin",
            "display_name": "Dragon Skin",
            "description": null,
            "asset_kind": "skin",
            "ownership_model": "entitlement",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let first_grant = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/entitlements/dragon_skin/grant",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "reasonText": "reward" }),
        )
        .await;
    assert!(first_grant.status().is_success());

    let second_grant = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/entitlements/dragon_skin/grant",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "reasonText": "duplicate grant" }),
        )
        .await;
    assert!(second_grant.status().is_success());

    let entitlements = app
        .get_json("/api/user/me/inventory/entitlements", &user.access_token)
        .await;
    assert!(entitlements.status().is_success());
    let entitlements_body: serde_json::Value =
        entitlements.json().await.expect("entitlements json");
    assert_eq!(entitlements_body.as_array().expect("array").len(), 1);
    assert_eq!(entitlements_body[0]["assetKey"], "dragon_skin");
    assert_eq!(
        app.inventory_operation_count("entitlement_granted").await,
        1
    );

    let revoke = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/entitlements/dragon_skin/revoke",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "reasonText": "cleanup" }),
        )
        .await;
    assert!(revoke.status().is_success());

    let after_revoke = app
        .get_json("/api/user/me/inventory/entitlements", &user.access_token)
        .await;
    let after_revoke_body: serde_json::Value = after_revoke
        .json()
        .await
        .expect("entitlements after revoke");
    assert!(after_revoke_body.as_array().expect("array").is_empty());
    assert_eq!(
        app.inventory_operation_count("entitlement_revoked").await,
        1
    );
}

#[tokio::test]
#[serial]
async fn stackable_flow_supports_add_remove_set_presence_and_history() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("StackAdmin", true, &[]).await;
    let user = app.issue_user_token("StackUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "event_ticket",
            "display_name": "Event Ticket",
            "description": null,
            "asset_kind": "ticket",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let add = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/event_ticket/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 5, "reasonText": "grant" }),
        )
        .await;
    assert!(add.status().is_success());

    let remove = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/event_ticket/remove",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 2, "reasonText": "consume" }),
        )
        .await;
    assert!(remove.status().is_success());

    let set_amount = app
        .put_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/event_ticket",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 10, "reasonText": "sync" }),
        )
        .await;
    assert!(set_amount.status().is_success());

    let presence = app
        .get_json(
            "/api/user/me/inventory/contains/event_ticket",
            &user.access_token,
        )
        .await;
    let presence_body: serde_json::Value = presence.json().await.expect("presence json");
    assert_eq!(presence_body["exists"], true);
    assert_eq!(presence_body["amount"], 10);

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables json");
    assert_eq!(stackables_body[0]["amount"], 10);

    let history = app
        .get_json(
            &format!("/api/admin/users/{}/inventory/history", user.user_id),
            &admin.access_token,
        )
        .await;
    let history_body: serde_json::Value = history.json().await.expect("history json");
    assert_eq!(history_body.as_array().expect("array").len(), 3);
    assert_eq!(app.inventory_operation_count("stackable_added").await, 1);
    assert_eq!(app.inventory_operation_count("stackable_removed").await, 1);
    assert_eq!(app.inventory_operation_count("stackable_set").await, 1);
}

#[tokio::test]
#[serial]
async fn stackable_remove_rejects_insufficient_amount() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("StackFailAdmin", true, &[]).await;
    let user = app.issue_user_token("StackFailUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "raid_key",
            "display_name": "Raid Key",
            "description": null,
            "asset_kind": "item",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let response = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/raid_key/remove",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 1, "reasonText": "consume" }),
        )
        .await;
    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
#[serial]
async fn expirable_active_prolong_accumulates_but_expired_restart_from_now_and_hidden_when_inactive()
 {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("ExpAdmin", true, &[]).await;
    let user = app.issue_user_token("ExpUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "premium_1d",
            "display_name": "Premium 1d",
            "description": null,
            "asset_kind": "subscription",
            "ownership_model": "expirable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let first = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/premium_1d/prolong",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 120 }),
        )
        .await;
    let first_body: serde_json::Value = first.json().await.expect("first prolong");
    let first_expires =
        chrono::DateTime::parse_from_rfc3339(first_body["expiresAt"].as_str().expect("expiresAt"))
            .expect("first expires")
            .with_timezone(&chrono::Utc);

    let second = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/premium_1d/prolong",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 120 }),
        )
        .await;
    let second_body: serde_json::Value = second.json().await.expect("second prolong");
    let second_expires =
        chrono::DateTime::parse_from_rfc3339(second_body["expiresAt"].as_str().expect("expiresAt"))
            .expect("second expires")
            .with_timezone(&chrono::Utc);
    assert!(second_expires >= first_expires + chrono::Duration::seconds(119));

    let past = chrono::Utc::now() - chrono::Duration::seconds(5);
    let set_past = app
        .put_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/premium_1d/expiration",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "expiresAt": past.to_rfc3339() }),
        )
        .await;
    assert!(set_past.status().is_success());

    let inventory_after_past = app
        .get_json("/api/user/me/inventory", &user.access_token)
        .await;
    let inventory_after_past_body: serde_json::Value = inventory_after_past
        .json()
        .await
        .expect("inventory after past");
    assert!(
        inventory_after_past_body["expirables"]
            .as_array()
            .expect("exp array")
            .is_empty()
    );

    let before_restart = chrono::Utc::now();
    let restart = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/premium_1d/prolong",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 60 }),
        )
        .await;
    let restart_body: serde_json::Value = restart.json().await.expect("restart prolong");
    let restart_expires = chrono::DateTime::parse_from_rfc3339(
        restart_body["expiresAt"].as_str().expect("expiresAt"),
    )
    .expect("restart expires")
    .with_timezone(&chrono::Utc);
    assert!(restart_expires >= before_restart + chrono::Duration::seconds(59));
    assert!(restart_expires <= before_restart + chrono::Duration::seconds(70));
}

#[tokio::test]
#[serial]
async fn wallet_supports_credit_debit_adjustment_and_transaction_history() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("WalletAdmin", true, &[]).await;
    let user = app.issue_user_token("WalletUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "gold",
            "display_name": "Gold",
            "description": null,
            "asset_kind": "currency",
            "ownership_model": "stackable",
            "is_currency": true,
            "is_user_purchasable": false,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let credit = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/gold/credit", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 100, "reasonText": "grant" }),
        )
        .await;
    assert!(credit.status().is_success());

    let debit = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/gold/debit", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 30, "reasonText": "spend" }),
        )
        .await;
    assert!(debit.status().is_success());

    let adjust = app
        .put_json(
            &format!("/api/admin/users/{}/wallet/gold", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 250, "reasonText": "repair" }),
        )
        .await;
    assert!(adjust.status().is_success());
    let adjust_body: serde_json::Value = adjust.json().await.expect("adjust json");
    assert_eq!(adjust_body["balance"], 250);

    let my_wallet = app
        .get_json("/api/user/me/wallet/gold", &user.access_token)
        .await;
    let my_wallet_body: serde_json::Value = my_wallet.json().await.expect("wallet json");
    assert_eq!(my_wallet_body["balance"], 250);

    let txs = app
        .get_json(
            &format!("/api/admin/users/{}/wallet/gold/transactions", user.user_id),
            &admin.access_token,
        )
        .await;
    let txs_body: serde_json::Value = txs.json().await.expect("wallet txs");
    assert_eq!(txs_body.as_array().expect("array").len(), 3);
    assert_eq!(app.wallet_transaction_count("credit").await, 1);
    assert_eq!(app.wallet_transaction_count("debit").await, 1);
    assert_eq!(app.wallet_transaction_count("adjustment").await, 1);
}

#[tokio::test]
#[serial]
async fn wallet_operations_require_currency_assets_and_debit_checks_balance() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("WalletRuleAdmin", true, &[]).await;
    let user = app.issue_user_token("WalletRuleUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "supply_box",
            "display_name": "Supply Box",
            "description": null,
            "asset_kind": "lootbox",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "gems",
            "display_name": "Gems",
            "description": null,
            "asset_kind": "currency",
            "ownership_model": "stackable",
            "is_currency": true,
            "is_user_purchasable": false,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let non_currency_wallet = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/supply_box/credit", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 10 }),
        )
        .await;
    assert_eq!(
        non_currency_wallet.status(),
        reqwest::StatusCode::BAD_REQUEST
    );

    let overdraft = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/gems/debit", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert_eq!(
        overdraft.status(),
        reqwest::StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
#[serial]
async fn default_wallet_shortcuts_support_balance_credit_debit_and_adjustment() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("DefaultWalletAdmin", true, &[]).await;
    let user = app.issue_user_token("DefaultWalletUser", false, &[]).await;

    let self_initial = app
        .get_json("/api/user/me/wallet/default", &user.access_token)
        .await;
    assert!(self_initial.status().is_success());
    let self_initial_body: serde_json::Value =
        self_initial.json().await.expect("self default wallet");
    assert_eq!(self_initial_body["currencyKey"], "coin_default");
    assert_eq!(self_initial_body["balance"], 0);

    let admin_initial = app
        .get_json(
            &format!("/api/admin/users/{}/wallet/default", user.user_id),
            &admin.access_token,
        )
        .await;
    assert!(admin_initial.status().is_success());
    let admin_initial_body: serde_json::Value =
        admin_initial.json().await.expect("admin default wallet");
    assert_eq!(admin_initial_body["currencyKey"], "coin_default");
    assert_eq!(admin_initial_body["balance"], 0);

    let credit = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/default/credit", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 120, "reasonText": "quest_reward" }),
        )
        .await;
    assert!(credit.status().is_success());
    let credit_body: serde_json::Value = credit.json().await.expect("credit json");
    assert_eq!(credit_body["currencyKey"], "coin_default");
    assert_eq!(credit_body["balance"], 120);

    let debit = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/default/debit", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 45, "reasonText": "purchase" }),
        )
        .await;
    assert!(debit.status().is_success());
    let debit_body: serde_json::Value = debit.json().await.expect("debit json");
    assert_eq!(debit_body["balance"], 75);

    let adjust = app
        .put_json(
            &format!("/api/admin/users/{}/wallet/default", user.user_id),
            &admin.access_token,
            serde_json::json!({ "amount": 500, "reasonText": "repair" }),
        )
        .await;
    assert!(adjust.status().is_success());
    let adjust_body: serde_json::Value = adjust.json().await.expect("adjust json");
    assert_eq!(adjust_body["currencyKey"], "coin_default");
    assert_eq!(adjust_body["balance"], 500);

    let self_after = app
        .get_json("/api/user/me/wallet/default", &user.access_token)
        .await;
    let self_after_body: serde_json::Value =
        self_after.json().await.expect("self default wallet after");
    assert_eq!(self_after_body["balance"], 500);

    let txs = app
        .get_json(
            &format!(
                "/api/admin/users/{}/wallet/coin_default/transactions",
                user.user_id
            ),
            &admin.access_token,
        )
        .await;
    let txs_body: serde_json::Value = txs.json().await.expect("default wallet txs");
    assert_eq!(txs_body.as_array().expect("tx array").len(), 3);
}

#[tokio::test]
#[serial]
async fn default_wallet_balance_returns_zero_without_balance_row() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("DefaultZeroAdmin", true, &[]).await;
    let user = app.issue_user_token("DefaultZeroUser", false, &[]).await;

    let self_balance = app
        .get_json("/api/user/me/wallet/default", &user.access_token)
        .await;
    assert!(self_balance.status().is_success());
    let self_body: serde_json::Value = self_balance.json().await.expect("self default balance");
    assert_eq!(self_body["currencyKey"], "coin_default");
    assert_eq!(self_body["balance"], 0);

    let admin_balance = app
        .get_json(
            &format!("/api/admin/users/{}/wallet/default", user.user_id),
            &admin.access_token,
        )
        .await;
    assert!(admin_balance.status().is_success());
    let admin_body: serde_json::Value = admin_balance.json().await.expect("admin default balance");
    assert_eq!(admin_body["currencyKey"], "coin_default");
    assert_eq!(admin_body["balance"], 0);

    let txs = app
        .get_json(
            &format!(
                "/api/admin/users/{}/wallet/coin_default/transactions",
                user.user_id
            ),
            &admin.access_token,
        )
        .await;
    assert!(txs.status().is_success());
    let txs_body: serde_json::Value = txs.json().await.expect("default tx history");
    assert_eq!(txs_body.as_array().expect("tx array").len(), 0);
}

#[tokio::test]
#[serial]
async fn subscription_shortcuts_report_status_and_reject_pro_while_plus_is_active() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SubscriptionAdmin", true, &[]).await;
    let user = app.issue_user_token("SubscriptionUser", false, &[]).await;

    let initial = app
        .get_json("/api/user/me/subscription/status", &user.access_token)
        .await;
    assert!(initial.status().is_success());
    let initial_body: serde_json::Value =
        initial.json().await.expect("initial subscription status");
    assert_eq!(initial_body["status"], "none");

    let plus = app
        .post_json(
            &format!(
                "/api/admin/users/{}/subscriptions/plus/prolong",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 3600, "reasonText": "plus_grant" }),
        )
        .await;
    assert!(plus.status().is_success());
    let plus_body: serde_json::Value = plus.json().await.expect("plus json");
    assert_eq!(plus_body["assetKey"], "subscription_plus");

    let plus_status = app
        .get_json("/api/user/me/subscription/status", &user.access_token)
        .await;
    let plus_status_body: serde_json::Value = plus_status.json().await.expect("plus status");
    assert_eq!(plus_status_body["status"], "plus");

    let pro_conflict = app
        .post_json(
            &format!(
                "/api/admin/users/{}/subscriptions/pro/prolong",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 3600, "reasonText": "pro_upgrade_attempt" }),
        )
        .await;
    assert_eq!(pro_conflict.status(), reqwest::StatusCode::BAD_REQUEST);
    let pro_conflict_body: serde_json::Value =
        pro_conflict.json().await.expect("pro conflict json");
    assert!(
        pro_conflict_body["error"]
            .as_str()
            .expect("error message")
            .contains("subscription_plus")
    );

    let pro_asset = app
        .asset_by_key("subscription_pro")
        .await
        .expect("subscription_pro asset");
    let now = chrono::Utc::now();

    auth::entities::UserExpirableAssetActiveModel {
        user_id: Set(uuid::Uuid::parse_str(&user.user_id).expect("user uuid")),
        asset_definition_id: Set(pro_asset.id),
        expires_at: Set(now + chrono::Duration::hours(4)),
        granted_at: Set(now),
        last_extended_at: Set(Some(now)),
        granted_by_actor: Set("{\"kind\":\"admin\"}".to_string()),
        updated_at: Set(now),
    }
    .insert(&app.db)
    .await
    .expect("insert pro raw row");

    let admin_status = app
        .get_json(
            &format!("/api/admin/users/{}/subscription/status", user.user_id),
            &admin.access_token,
        )
        .await;
    assert!(admin_status.status().is_success());
    let admin_status_body: serde_json::Value = admin_status.json().await.expect("admin status");
    assert_eq!(admin_status_body["status"], "pro");
}

#[tokio::test]
#[serial]
async fn subscription_status_reports_none_plus_pro_and_pro_wins_over_double_state() {
    let app = TestApp::spawn().await;
    let admin = app
        .issue_user_token("SubscriptionMatrixAdmin", true, &[])
        .await;

    let none_user = app.issue_user_token("SubscriptionNone", false, &[]).await;
    let plus_user = app.issue_user_token("SubscriptionPlus", false, &[]).await;
    let pro_user = app.issue_user_token("SubscriptionPro", false, &[]).await;
    let both_user = app.issue_user_token("SubscriptionBoth", false, &[]).await;

    let plus_response = app
        .post_json(
            &format!(
                "/api/admin/users/{}/subscriptions/plus/prolong",
                plus_user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 3600 }),
        )
        .await;
    assert!(plus_response.status().is_success());

    let pro_response = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/subscription_pro/prolong",
                pro_user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 3600 }),
        )
        .await;
    assert!(pro_response.status().is_success());

    let both_plus = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/subscription_plus/prolong",
                both_user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 3600 }),
        )
        .await;
    assert!(both_plus.status().is_success());
    let both_pro = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/expirables/subscription_pro/prolong",
                both_user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "durationSeconds": 3600 }),
        )
        .await;
    assert!(both_pro.status().is_success());

    let none_status = app
        .get_json("/api/user/me/subscription/status", &none_user.access_token)
        .await;
    let none_body: serde_json::Value = none_status.json().await.expect("none status");
    assert_eq!(none_body["status"], "none");

    let plus_status = app
        .get_json("/api/user/me/subscription/status", &plus_user.access_token)
        .await;
    let plus_body: serde_json::Value = plus_status.json().await.expect("plus status");
    assert_eq!(plus_body["status"], "plus");

    let pro_status = app
        .get_json("/api/user/me/subscription/status", &pro_user.access_token)
        .await;
    let pro_body: serde_json::Value = pro_status.json().await.expect("pro status");
    assert_eq!(pro_body["status"], "pro");

    let both_status = app
        .get_json("/api/user/me/subscription/status", &both_user.access_token)
        .await;
    let both_body: serde_json::Value = both_status.json().await.expect("both status");
    assert_eq!(both_body["status"], "pro");
}

#[tokio::test]
#[serial]
async fn missing_user_skin_without_default_returns_not_found() {
    let app = TestApp::spawn().await;
    let missing_user_id = uuid::Uuid::new_v4().to_string();

    let response = app
        .get_bytes_without_auth(&format!("/api/skins/{missing_user_id}"))
        .await;

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn missing_user_skin_falls_back_to_admin_default_skin() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("DefaultSkinAdmin", true, &[]).await;
    let fallback_png = make_skin_png([255, 0, 0, 255]);

    let upload = app
        .post_multipart(
            "/api/admin/skins/default?model=default",
            &admin.access_token,
            "default.png",
            "image/png",
            fallback_png.clone(),
        )
        .await;
    assert!(
        upload.status().is_success(),
        "{}",
        upload.text().await.unwrap_or_default()
    );

    let metadata = app
        .get_json("/api/admin/skins/default", &admin.access_token)
        .await;
    assert!(metadata.status().is_success());
    let metadata_body: serde_json::Value = metadata.json().await.expect("default skin metadata");
    assert_eq!(metadata_body["imageUrl"], "/api/skins/default");
    assert_eq!(app.audit_log_count("admin.default_skin.updated").await, 1);

    let missing_user_id = uuid::Uuid::new_v4().to_string();
    let response = app
        .get_bytes_without_auth(&format!("/api/skins/{missing_user_id}"))
        .await;
    assert!(response.status().is_success());
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
    let response_bytes = response.bytes().await.expect("fallback bytes");

    let direct_default = app.get_bytes_without_auth("/api/skins/default").await;
    assert!(direct_default.status().is_success());
    let direct_default_bytes = direct_default.bytes().await.expect("default bytes");

    assert_eq!(response_bytes.as_ref(), direct_default_bytes.as_ref());
}

#[tokio::test]
#[serial]
async fn personal_skin_takes_priority_over_default_skin() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SkinPriorityAdmin", true, &[]).await;
    let user = app.issue_user_token("SkinOwner", false, &[]).await;
    let default_png = make_skin_png([0, 255, 0, 255]);
    let personal_png = make_skin_png([0, 0, 255, 255]);

    let upload_default = app
        .post_multipart(
            "/api/admin/skins/default?model=default",
            &admin.access_token,
            "default.png",
            "image/png",
            default_png,
        )
        .await;
    assert!(upload_default.status().is_success());

    let upload_personal = app
        .post_multipart(
            "/api/skins/me?model=slim",
            &user.access_token,
            "personal.png",
            "image/png",
            personal_png.clone(),
        )
        .await;
    assert!(
        upload_personal.status().is_success(),
        "{}",
        upload_personal.text().await.unwrap_or_default()
    );

    let response = app
        .get_bytes_without_auth(&format!("/api/skins/{}", user.user_id))
        .await;
    assert!(response.status().is_success());
    let response_bytes = response.bytes().await.expect("personal skin bytes");

    let default_response = app.get_bytes_without_auth("/api/skins/default").await;
    assert!(default_response.status().is_success());
    let default_bytes = default_response.bytes().await.expect("default skin bytes");

    assert_ne!(response_bytes.as_ref(), default_bytes.as_ref());
}

#[tokio::test]
#[serial]
async fn admin_can_issue_service_token_and_view_separate_audit() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SvcAdmin", true, &[]).await;

    let create_response = app
        .post_json(
            "/api/admin/service-tokens",
            &admin.access_token,
            serde_json::json!({
                "systemName": "matchmaker",
                "description": "Internal matchmaker service"
            }),
        )
        .await;
    assert!(
        create_response.status().is_success(),
        "{}",
        create_response.text().await.unwrap_or_default()
    );
    let created: serde_json::Value = create_response
        .json()
        .await
        .expect("create service token json");
    let token_id = created["id"].as_str().expect("token id");
    let plaintext = created["plaintextToken"].as_str().expect("plaintext token");
    assert!(plaintext.starts_with("svcs_"));

    let list_response = app
        .get_json("/api/admin/service-tokens", &admin.access_token)
        .await;
    assert!(list_response.status().is_success());
    let list_body: serde_json::Value = list_response
        .json()
        .await
        .expect("list service tokens json");
    let items = list_body.as_array().expect("service token list");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["systemName"], "matchmaker");
    assert!(items[0]["plaintextToken"].is_null());

    let audit_response = app
        .get_json(
            &format!("/api/admin/service-tokens/{token_id}/audit"),
            &admin.access_token,
        )
        .await;
    assert!(audit_response.status().is_success());
    let audit_body: serde_json::Value = audit_response
        .json()
        .await
        .expect("service token audit json");
    assert_eq!(audit_body.as_array().expect("audit array").len(), 1);
    assert_eq!(audit_body[0]["action"], "service_token.created");
    assert_eq!(audit_body[0]["metadata"]["systemName"], "matchmaker");
}

#[tokio::test]
#[serial]
async fn service_token_can_mutate_wallet_and_inventory_as_system_actor() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SvcFlowAdmin", true, &[]).await;
    let user = app.issue_user_token("SvcFlowUser", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "silver",
            "display_name": "Silver",
            "description": null,
            "asset_kind": "currency",
            "ownership_model": "stackable",
            "is_currency": true,
            "is_user_purchasable": false,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "arena_ticket",
            "display_name": "Arena Ticket",
            "description": null,
            "asset_kind": "ticket",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let created = app
        .post_json(
            "/api/admin/service-tokens",
            &admin.access_token,
            serde_json::json!({
                "systemName": "reward_daemon",
                "description": "Reward distributor"
            }),
        )
        .await
        .json::<serde_json::Value>()
        .await
        .expect("service token create body");
    let service_token = created["plaintextToken"]
        .as_str()
        .expect("service token")
        .to_string();

    let credit_response = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/silver/credit", user.user_id),
            &service_token,
            serde_json::json!({ "amount": 50, "reasonText": "reward" }),
        )
        .await;
    assert!(
        credit_response.status().is_success(),
        "{}",
        credit_response.text().await.unwrap_or_default()
    );

    let add_stackable_response = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/arena_ticket/add",
                user.user_id
            ),
            &service_token,
            serde_json::json!({ "amount": 3, "reasonText": "grant" }),
        )
        .await;
    assert!(
        add_stackable_response.status().is_success(),
        "{}",
        add_stackable_response.text().await.unwrap_or_default()
    );

    let wallet_txs = app
        .get_json(
            &format!(
                "/api/admin/users/{}/wallet/silver/transactions",
                user.user_id
            ),
            &admin.access_token,
        )
        .await;
    let wallet_txs_body: serde_json::Value = wallet_txs.json().await.expect("wallet tx json");
    assert_eq!(wallet_txs_body[0]["actorKind"], "system");
    assert_eq!(wallet_txs_body[0]["actorServiceName"], "reward_daemon");

    let inventory_history = app
        .get_json(
            &format!("/api/admin/users/{}/inventory/history", user.user_id),
            &admin.access_token,
        )
        .await;
    let inventory_history_body: serde_json::Value = inventory_history
        .json()
        .await
        .expect("inventory history json");
    assert_eq!(inventory_history_body[0]["actorKind"], "system");
    assert_eq!(
        inventory_history_body[0]["actorServiceName"],
        "reward_daemon"
    );
}

#[tokio::test]
#[serial]
async fn service_token_rotation_and_revoke_change_validity() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("SvcRotateAdmin", true, &[]).await;

    let user = app.issue_user_token("SvcRotateUser", false, &[]).await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "credits",
            "display_name": "Credits",
            "description": null,
            "asset_kind": "currency",
            "ownership_model": "stackable",
            "is_currency": true,
            "is_user_purchasable": false,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let created = app
        .post_json(
            "/api/admin/service-tokens",
            &admin.access_token,
            serde_json::json!({ "systemName": "daily_rewards" }),
        )
        .await;
    assert!(created.status().is_success());
    let created_body: serde_json::Value = created.json().await.expect("created token json");
    let token_id = created_body["id"].as_str().expect("token id").to_string();
    let old_token = created_body["plaintextToken"]
        .as_str()
        .expect("old plaintext token")
        .to_string();

    let rotate_response = app
        .post_json(
            &format!("/api/admin/service-tokens/{token_id}/rotate"),
            &admin.access_token,
            serde_json::json!({ "reason": "routine rotation" }),
        )
        .await;
    assert!(
        rotate_response.status().is_success(),
        "{}",
        rotate_response.text().await.unwrap_or_default()
    );
    let rotated_body: serde_json::Value = rotate_response.json().await.expect("rotated token json");
    let new_token = rotated_body["plaintextToken"]
        .as_str()
        .expect("new plaintext token")
        .to_string();
    assert_ne!(old_token, new_token);

    let old_use = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/credits/credit", user.user_id),
            &old_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert_eq!(old_use.status(), reqwest::StatusCode::FORBIDDEN);

    let new_use = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/credits/credit", user.user_id),
            &new_token,
            serde_json::json!({ "amount": 7 }),
        )
        .await;
    assert!(new_use.status().is_success());

    let revoke_response = app
        .post_json(
            &format!(
                "/api/admin/service-tokens/{}/revoke",
                rotated_body["id"].as_str().expect("new token id")
            ),
            &admin.access_token,
            serde_json::json!({ "reason": "disabled" }),
        )
        .await;
    assert!(revoke_response.status().is_success());

    let after_revoke = app
        .post_json(
            &format!("/api/admin/users/{}/wallet/credits/credit", user.user_id),
            &new_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert_eq!(after_revoke.status(), reqwest::StatusCode::FORBIDDEN);

    let audit_response = app
        .get_json(
            &format!(
                "/api/admin/service-tokens/{}/audit",
                rotated_body["id"].as_str().expect("new token id")
            ),
            &admin.access_token,
        )
        .await;
    assert!(audit_response.status().is_success());
    let audit_body: serde_json::Value = audit_response.json().await.expect("rotated audit json");
    let actions: Vec<_> = audit_body
        .as_array()
        .expect("audit array")
        .iter()
        .map(|item| item["action"].as_str().expect("action"))
        .collect();
    assert!(actions.contains(&"service_token.created"));
    assert!(actions.contains(&"service_token.revoked"));
}

#[tokio::test]
#[serial]
async fn lootbox_catalog_and_user_open_flow_work_with_exact_drop_payloads() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LootboxAdmin", true, &[]).await;
    let user = app.issue_user_token("LootboxUser", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "starter_case",
            "display_name": "Starter Case",
            "description": "Starter lootbox",
            "asset_kind": "lootbox",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "repair_kit",
            "display_name": "Repair Kit",
            "description": null,
            "asset_kind": "item",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let create_lootbox = app
        .post_json(
            "/api/admin/lootboxes",
            &admin.access_token,
            serde_json::json!({
                "asset_key": "starter_case",
                "is_active": true,
                "metadata": { "theme": "starter" }
            }),
        )
        .await;
    assert!(create_lootbox.status().is_success());
    let lootbox_body: serde_json::Value = create_lootbox.json().await.expect("lootbox json");
    let lootbox_id = lootbox_body["definition"]["id"]
        .as_str()
        .expect("lootbox id")
        .to_string();

    let patch_lootbox = app
        .patch_json(
            &format!("/api/admin/lootboxes/{lootbox_id}"),
            &admin.access_token,
            serde_json::json!({
                "displayName": "Updated Starter Case",
                "description": "Updated starter lootbox",
                "isPublic": false
            }),
        )
        .await;
    assert!(
        patch_lootbox.status().is_success(),
        "{}",
        patch_lootbox.text().await.unwrap_or_default()
    );
    let patched_body: serde_json::Value = patch_lootbox.json().await.expect("patched lootbox");
    assert_eq!(
        patched_body["definition"]["assetDisplayName"],
        "Updated Starter Case"
    );
    assert_eq!(
        patched_body["definition"]["assetDescription"],
        "Updated starter lootbox"
    );
    assert_eq!(patched_body["definition"]["isPublic"], false);

    let hidden_public_view = app
        .get_without_auth(&format!("/api/lootboxes/{lootbox_id}"))
        .await;
    assert_eq!(hidden_public_view.status(), reqwest::StatusCode::NOT_FOUND);

    let restore_public = app
        .patch_json(
            &format!("/api/admin/lootboxes/{lootbox_id}"),
            &admin.access_token,
            serde_json::json!({ "isPublic": true }),
        )
        .await;
    assert!(
        restore_public.status().is_success(),
        "{}",
        restore_public.text().await.unwrap_or_default()
    );

    let add_drop = app
        .post_json(
            &format!("/api/admin/lootboxes/{lootbox_id}/drops"),
            &admin.access_token,
            serde_json::json!({
                "reward_asset_key": "repair_kit",
                "amount": 2,
                "weight": 1,
                "title_i18n": { "ru": "2 ремонтных набора", "en": "2 Repair Kits" },
                "is_active": true,
                "sort_order": 10
            }),
        )
        .await;
    assert!(add_drop.status().is_success());

    let asset_key_public_view = app.get_without_auth("/api/lootboxes/starter_case").await;
    assert_eq!(asset_key_public_view.status(), reqwest::StatusCode::BAD_REQUEST);

    let public_view = app
        .get_without_auth(&format!("/api/lootboxes/{lootbox_id}"))
        .await;
    assert!(public_view.status().is_success());
    let public_body: serde_json::Value = public_view.json().await.expect("public lootbox");
    assert_eq!(public_body["definition"]["assetKey"], "starter_case");
    assert_eq!(public_body["drops"].as_array().expect("drops").len(), 1);
    assert_eq!(public_body["drops"][0]["amount"], 2);
    assert_eq!(public_body["drops"][0]["totalWeight"], 1);

    let grant_case = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/starter_case/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 1, "reasonText": "test grant" }),
        )
        .await;
    assert!(grant_case.status().is_success());

    let open = app
        .post_json(
            "/api/user/me/lootboxes/starter_case/open?feedLength=12&locale=ru",
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(
        open.status().is_success(),
        "{}",
        open.text().await.unwrap_or_default()
    );
    let open_body: serde_json::Value = open.json().await.expect("open body");
    assert_eq!(open_body["lootboxAssetKey"], "starter_case");
    assert_eq!(open_body["reward"]["assetKey"], "repair_kit");
    assert_eq!(open_body["reward"]["amount"], 2);
    assert_eq!(open_body["reward"]["title"], "2 ремонтных набора");
    assert_eq!(open_body["feed"].as_array().expect("feed").len(), 12);
    assert!(open_body["winnerIndex"].as_u64().expect("winner index") < 12);

    let owned_after = app
        .get_json("/api/user/me/lootboxes", &user.access_token)
        .await;
    assert!(owned_after.status().is_success());
    let owned_after_body: serde_json::Value = owned_after.json().await.expect("owned after");
    assert!(owned_after_body.as_array().expect("owned array").is_empty());

    let stackables = app
        .get_json("/api/user/me/inventory/stackables", &user.access_token)
        .await;
    let stackables_body: serde_json::Value = stackables.json().await.expect("stackables");
    assert_eq!(stackables_body[0]["assetKey"], "repair_kit");
    assert_eq!(stackables_body[0]["amount"], 2);

    let history = app
        .get_json("/api/user/me/lootboxes/open-history", &user.access_token)
        .await;
    assert!(history.status().is_success());
    let history_body: serde_json::Value = history.json().await.expect("history");
    assert_eq!(history_body.as_array().expect("history array").len(), 1);
    assert_eq!(history_body[0]["actorKind"], "user");
    assert_eq!(app.audit_log_count("user.lootbox.opened").await, 1);
}

#[tokio::test]
#[serial]
async fn service_token_can_open_lootbox_for_user_and_history_keeps_service_actor() {
    let app = TestApp::spawn().await;
    let admin = app.issue_user_token("LootboxSvcAdmin", true, &[]).await;
    let user = app.issue_user_token("LootboxSvcUser", false, &[]).await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "service_case",
            "display_name": "Service Case",
            "description": null,
            "asset_kind": "lootbox",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": false,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "arena_ticket",
            "display_name": "Arena Ticket",
            "description": null,
            "asset_kind": "ticket",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let created_lootbox = app
        .post_json(
            "/api/admin/lootboxes",
            &admin.access_token,
            serde_json::json!({ "asset_key": "service_case" }),
        )
        .await
        .json::<serde_json::Value>()
        .await
        .expect("lootbox create");
    let lootbox_id = created_lootbox["definition"]["id"]
        .as_str()
        .expect("lootbox id");

    let add_drop = app
        .post_json(
            &format!("/api/admin/lootboxes/{lootbox_id}/drops"),
            &admin.access_token,
            serde_json::json!({
                "reward_asset_key": "arena_ticket",
                "amount": 1,
                "weight": 1,
                "title_i18n": { "en": "Arena Ticket" }
            }),
        )
        .await;
    assert!(add_drop.status().is_success());

    let grant_case = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/service_case/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert!(grant_case.status().is_success());

    let created_token = app
        .post_json(
            "/api/admin/service-tokens",
            &admin.access_token,
            serde_json::json!({ "systemName": "minecraft_plugin" }),
        )
        .await
        .json::<serde_json::Value>()
        .await
        .expect("service token");
    let service_token = created_token["plaintextToken"]
        .as_str()
        .expect("plaintext token");

    let inventory = app
        .get_json(
            &format!("/api/admin/users/{}/inventory", user.user_id),
            service_token,
        )
        .await;
    assert!(
        inventory.status().is_success(),
        "{}",
        inventory.text().await.unwrap_or_default()
    );
    let inventory_body: serde_json::Value = inventory.json().await.expect("service inventory");
    assert_eq!(inventory_body["stackables"][0]["assetKey"], "service_case");

    let list_lootboxes = app.get_json("/api/admin/lootboxes", service_token).await;
    assert!(
        list_lootboxes.status().is_success(),
        "{}",
        list_lootboxes.text().await.unwrap_or_default()
    );
    let list_lootboxes_body: serde_json::Value =
        list_lootboxes.json().await.expect("service lootbox list");
    assert!(
        list_lootboxes_body
            .as_array()
            .expect("lootbox list")
            .iter()
            .any(|item| item["id"] == lootbox_id)
    );

    let get_lootbox = app
        .get_json(&format!("/api/admin/lootboxes/{lootbox_id}"), service_token)
        .await;
    assert!(
        get_lootbox.status().is_success(),
        "{}",
        get_lootbox.text().await.unwrap_or_default()
    );

    let patch_lootbox = app
        .patch_json(
            &format!("/api/admin/lootboxes/{lootbox_id}"),
            service_token,
            serde_json::json!({ "metadata": { "managedBy": "minecraft_plugin" } }),
        )
        .await;
    assert!(
        patch_lootbox.status().is_success(),
        "{}",
        patch_lootbox.text().await.unwrap_or_default()
    );
    let patch_lootbox_body: serde_json::Value =
        patch_lootbox.json().await.expect("service lootbox patch");
    assert_eq!(
        patch_lootbox_body["definition"]["metadata"]["managedBy"],
        "minecraft_plugin"
    );

    let open = app
        .post_json(
            &format!(
                "/api/admin/users/{}/lootboxes/service_case/open?feedLength=25",
                user.user_id
            ),
            service_token,
            serde_json::json!({}),
        )
        .await;
    assert!(
        open.status().is_success(),
        "{}",
        open.text().await.unwrap_or_default()
    );
    let open_body: serde_json::Value = open.json().await.expect("service open");
    assert_eq!(open_body["reward"]["assetKey"], "arena_ticket");
    assert_eq!(open_body["feed"].as_array().expect("feed").len(), 25);

    let history = app
        .get_json(
            &format!("/api/admin/users/{}/lootboxes/open-history", user.user_id),
            service_token,
        )
        .await;
    assert!(history.status().is_success());
    let history_body: serde_json::Value = history.json().await.expect("admin history");
    assert_eq!(history_body.as_array().expect("history").len(), 1);
    assert_eq!(history_body[0]["actorKind"], "system");
    assert_eq!(history_body[0]["actorServiceName"], "minecraft_plugin");

    let all_history = app
        .get_json("/api/admin/lootboxes/open-history", service_token)
        .await;
    assert!(
        all_history.status().is_success(),
        "{}",
        all_history.text().await.unwrap_or_default()
    );
    let all_history_body: serde_json::Value = all_history.json().await.expect("global history");
    assert_eq!(all_history_body.as_array().expect("history").len(), 1);
    assert_eq!(
        app.audit_log_count("admin.lootbox.opened_for_user").await,
        1
    );
}

#[tokio::test]
#[serial]
async fn lootbox_can_grant_expirable_reward_and_report_expiration() {
    let app = TestApp::spawn().await;
    let admin = app
        .issue_user_token("LootboxExpirableAdmin", true, &[])
        .await;
    let user = app
        .issue_user_token("LootboxExpirableUser", false, &[])
        .await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "subscription_case",
            "display_name": "Subscription Case",
            "description": null,
            "asset_kind": "lootbox",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let created_lootbox = app
        .post_json(
            "/api/admin/lootboxes",
            &admin.access_token,
            serde_json::json!({ "asset_key": "subscription_case" }),
        )
        .await
        .json::<serde_json::Value>()
        .await
        .expect("lootbox create");
    let lootbox_id = created_lootbox["definition"]["id"]
        .as_str()
        .expect("lootbox id");

    let add_drop = app
        .post_json(
            &format!("/api/admin/lootboxes/{lootbox_id}/drops"),
            &admin.access_token,
            serde_json::json!({
                "reward_asset_key": "subscription_plus",
                "duration_seconds": 3600,
                "weight": 1,
                "title_i18n": { "ru": "Плюс на 1 час" }
            }),
        )
        .await;
    assert!(add_drop.status().is_success());

    let grant_case = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/subscription_case/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert!(grant_case.status().is_success());

    let open = app
        .post_json(
            "/api/user/me/lootboxes/subscription_case/open?locale=ru",
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(open.status().is_success());
    let open_body: serde_json::Value = open.json().await.expect("open expirable");
    assert_eq!(open_body["reward"]["assetKey"], "subscription_plus");
    assert_eq!(open_body["reward"]["ownershipModel"], "expirable");
    assert_eq!(open_body["reward"]["durationSeconds"], 3600);
    assert_eq!(open_body["reward"]["title"], "Плюс на 1 час");
    assert!(open_body["reward"]["expiresAt"].is_string());

    let expirables = app
        .get_json(
            "/api/user/me/inventory/expirables/active",
            &user.access_token,
        )
        .await;
    assert!(expirables.status().is_success());
    let expirables_body: serde_json::Value = expirables.json().await.expect("expirables");
    assert_eq!(
        expirables_body.as_array().expect("expirables array").len(),
        1
    );
    assert_eq!(expirables_body[0]["assetKey"], "subscription_plus");
    assert!(expirables_body[0]["isActive"].as_bool().expect("active"));
}

#[tokio::test]
#[serial]
async fn lootbox_can_grant_currency_reward_to_wallet() {
    let app = TestApp::spawn().await;
    let admin = app
        .issue_user_token("LootboxCurrencyAdmin", true, &[])
        .await;
    let user = app
        .issue_user_token("LootboxCurrencyUser", false, &[])
        .await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "coin_case",
            "display_name": "Coin Case",
            "description": null,
            "asset_kind": "lootbox",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let created_lootbox = app
        .post_json(
            "/api/admin/lootboxes",
            &admin.access_token,
            serde_json::json!({ "asset_key": "coin_case" }),
        )
        .await
        .json::<serde_json::Value>()
        .await
        .expect("lootbox create");
    let lootbox_id = created_lootbox["definition"]["id"]
        .as_str()
        .expect("lootbox id");

    let add_drop = app
        .post_json(
            &format!("/api/admin/lootboxes/{lootbox_id}/drops"),
            &admin.access_token,
            serde_json::json!({
                "reward_asset_key": "coin_default",
                "amount": 75,
                "weight": 1,
                "title_i18n": { "en": "75 Coins" }
            }),
        )
        .await;
    assert!(
        add_drop.status().is_success(),
        "{}",
        add_drop.text().await.unwrap_or_default()
    );

    let grant_case = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/coin_case/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert!(grant_case.status().is_success());

    let open = app
        .post_json(
            "/api/user/me/lootboxes/coin_case/open?locale=en",
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(
        open.status().is_success(),
        "{}",
        open.text().await.unwrap_or_default()
    );
    let open_body: serde_json::Value = open.json().await.expect("open currency");
    assert_eq!(open_body["selectedReward"]["assetKey"], "coin_default");
    assert_eq!(open_body["reward"]["assetKey"], "coin_default");
    assert_eq!(open_body["reward"]["ownershipModel"], "stackable");
    assert_eq!(open_body["reward"]["amount"], 75);
    assert_eq!(open_body["wasCompensated"], false);

    let wallet = app
        .get_json("/api/user/me/wallet/default", &user.access_token)
        .await;
    assert!(wallet.status().is_success());
    let wallet_body: serde_json::Value = wallet.json().await.expect("default wallet");
    assert_eq!(wallet_body["currencyKey"], "coin_default");
    assert_eq!(wallet_body["balance"], 75);
}

#[tokio::test]
#[serial]
async fn lootbox_duplicate_entitlement_grants_default_currency_compensation() {
    let app = TestApp::spawn().await;
    let admin = app
        .issue_user_token("LootboxDuplicateAdmin", true, &[])
        .await;
    let user = app
        .issue_user_token("LootboxDuplicateUser", false, &[])
        .await;

    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "cosmetic_case",
            "display_name": "Cosmetic Case",
            "description": null,
            "asset_kind": "lootbox",
            "ownership_model": "stackable",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;
    app.create_asset(
        &admin,
        serde_json::json!({
            "key": "crimson_banner",
            "display_name": "Crimson Banner",
            "description": null,
            "asset_kind": "cosmetic",
            "ownership_model": "entitlement",
            "is_currency": false,
            "is_user_purchasable": true,
            "is_public": true,
            "metadata": {}
        }),
    )
    .await;

    let created_lootbox = app
        .post_json(
            "/api/admin/lootboxes",
            &admin.access_token,
            serde_json::json!({ "asset_key": "cosmetic_case" }),
        )
        .await
        .json::<serde_json::Value>()
        .await
        .expect("lootbox create");
    let lootbox_id = created_lootbox["definition"]["id"]
        .as_str()
        .expect("lootbox id");

    let add_drop = app
        .post_json(
            &format!("/api/admin/lootboxes/{lootbox_id}/drops"),
            &admin.access_token,
            serde_json::json!({
                "reward_asset_key": "crimson_banner",
                "duplicate_compensation_amount": 125,
                "weight": 1,
                "title_i18n": { "en": "Crimson Banner" }
            }),
        )
        .await;
    assert!(
        add_drop.status().is_success(),
        "{}",
        add_drop.text().await.unwrap_or_default()
    );
    let add_drop_body: serde_json::Value = add_drop.json().await.expect("drop json");
    assert_eq!(add_drop_body["drops"][0]["duplicateCompensationAmount"], 125);

    let grant_entitlement = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/entitlements/crimson_banner/grant",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(grant_entitlement.status().is_success());

    let grant_case = app
        .post_json(
            &format!(
                "/api/admin/users/{}/inventory/stackables/cosmetic_case/add",
                user.user_id
            ),
            &admin.access_token,
            serde_json::json!({ "amount": 1 }),
        )
        .await;
    assert!(grant_case.status().is_success());

    let open = app
        .post_json(
            "/api/user/me/lootboxes/cosmetic_case/open?feedLength=8&locale=en",
            &user.access_token,
            serde_json::json!({}),
        )
        .await;
    assert!(
        open.status().is_success(),
        "{}",
        open.text().await.unwrap_or_default()
    );
    let open_body: serde_json::Value = open.json().await.expect("open duplicate");
    assert_eq!(open_body["selectedReward"]["assetKey"], "crimson_banner");
    assert_eq!(open_body["selectedReward"]["ownershipModel"], "entitlement");
    assert_eq!(open_body["reward"]["assetKey"], "coin_default");
    assert_eq!(open_body["reward"]["amount"], 125);
    assert_eq!(open_body["wasCompensated"], true);

    let winner_index = open_body["winnerIndex"].as_u64().expect("winner index") as usize;
    assert_eq!(open_body["feed"][winner_index]["assetKey"], "crimson_banner");

    let wallet = app
        .get_json("/api/user/me/wallet/default", &user.access_token)
        .await;
    assert!(wallet.status().is_success());
    let wallet_body: serde_json::Value = wallet.json().await.expect("default wallet");
    assert_eq!(wallet_body["balance"], 125);

    let history = app
        .get_json("/api/user/me/lootboxes/open-history", &user.access_token)
        .await;
    assert!(history.status().is_success());
    let history_body: serde_json::Value = history.json().await.expect("history");
    assert_eq!(history_body[0]["selectedReward"]["assetKey"], "crimson_banner");
    assert_eq!(history_body[0]["reward"]["assetKey"], "coin_default");
    assert_eq!(history_body[0]["reward"]["amount"], 125);
    assert_eq!(history_body[0]["wasCompensated"], true);
}
