use serde_json::Value;

const SITE_URL: &str = "https://svocraft.xyz";
const SQUADS_URL: &str = "https://svocraft.xyz/profile?tab=squads";
const SHOP_URL: &str = "https://svocraft.xyz/profile";

#[derive(Clone, Debug)]
pub struct DiscordTemplate {
    pub title: String,
    pub description: String,
    pub url: String,
    pub footer_text: String,
}

pub fn admin_custom(message: String) -> DiscordTemplate {
    DiscordTemplate {
        title: "✨ Уведомление от SVO".to_string(),
        description: message,
        url: SITE_URL.to_string(),
        footer_text: format!("🌐 Тыкай сюда: {SITE_URL}"),
    }
}

pub fn squad_invite_received(squad_name: &str, inviter_username: &str) -> DiscordTemplate {
    DiscordTemplate {
        title: "💌 Приглашение в сквадик".to_string(),
        description: format!(
            "Тебя позвали в сквад **\"{squad_name}\"**~\n\
            Приглашение отправил: **{inviter_username}**.\n\
            Загляни на страницу сквадов, чтобы принять его или вежливо отказаться ✨"
        ),
        url: SQUADS_URL.to_string(),
        footer_text: format!("👥 Открыть сквадики: {SQUADS_URL}"),
    }
}

pub fn squad_kicked(squad_name: &str, leader_username: &str) -> DiscordTemplate {
    DiscordTemplate {
        title: "💔 Исключение из сквада".to_string(),
        description: format!(
            "Похоже, тебя исключили из сквада **\"{squad_name}\"**...\n\
            Это сделал лидер: **{leader_username}**."
        ),
        url: SQUADS_URL.to_string(),
        footer_text: format!("👥 Открыть страницу сквадов: {SQUADS_URL}"),
    }
}

pub fn shop_purchase_completed(product_name: &str, quantity: i64) -> DiscordTemplate {
    let description = if quantity > 1 {
        format!(
            "Покупка прошла успешно, ура-ура~ ✨\n\
            Ты получил: **{product_name} x{quantity}**."
        )
    } else {
        format!(
            "Покупка прошла успешно, ура-ура~ ✨\n\
            Ты получил: **{product_name}**."
        )
    };

    DiscordTemplate {
        title: "🛍️ Покупка завершена".to_string(),
        description,
        url: SHOP_URL.to_string(),
        footer_text: format!("🌸 Открыть профиль: {SHOP_URL}"),
    }
}

pub fn render(template_key: &str, message: &str, metadata: &Value) -> DiscordTemplate {
    match template_key {
        "squad.invite_received" => squad_invite_received(
            metadata
                .get("squadName")
                .and_then(Value::as_str)
                .unwrap_or("неизвестный сквадик"),
            metadata
                .get("inviterUsername")
                .and_then(Value::as_str)
                .unwrap_or("неизвестный игрок"),
        ),
        "squad.kicked" => squad_kicked(
            metadata
                .get("squadName")
                .and_then(Value::as_str)
                .unwrap_or("неизвестный сквадик"),
            metadata
                .get("leaderUsername")
                .and_then(Value::as_str)
                .unwrap_or("неизвестный лидер"),
        ),
        "shop.purchase_completed" => shop_purchase_completed(
            metadata
                .get("productName")
                .and_then(Value::as_str)
                .unwrap_or("неизвестная покупочка"),
            metadata.get("quantity").and_then(Value::as_i64).unwrap_or(1),
        ),
        _ => admin_custom(message.to_string()),
    }
}