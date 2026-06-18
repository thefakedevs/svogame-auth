use crate::entities::{ShopOrderModel, ShopReceiptModel};

pub const TEMPLATE_SHOP_RECEIPT: &str = "shop.receipt";

#[derive(Clone, Debug)]
pub struct EmailTemplate {
    pub subject: String,
    pub html_body: String,
}

pub fn shop_receipt(order: &ShopOrderModel, receipt: &ShopReceiptModel) -> EmailTemplate {
    let receipt_url = receipt.print_url.as_deref().unwrap_or("#");
    let order_id = order.id.to_string();
    render_shop_receipt(ShopReceiptTemplateData {
        subject: format!("Чек по заказу #{order_id} на SvoCraft"),
        product_name: order.product_name.clone(),
        order_id,
        total_price_rub: order.total_price_rub,
        receipt_uuid: receipt
            .receipt_uuid
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        receipt_url: receipt_url.to_string(),
    })
}

pub fn test_shop_receipt(receipt_print_url: &str) -> EmailTemplate {
    render_shop_receipt(ShopReceiptTemplateData {
        subject: "Тестовая отправка чека SVO".to_string(),
        product_name: "Тестовая покупка SVO".to_string(),
        order_id: "test-order".to_string(),
        total_price_rub: 0,
        receipt_uuid: "test-receipt".to_string(),
        receipt_url: receipt_print_url.to_string(),
    })
}

struct ShopReceiptTemplateData {
    subject: String,
    product_name: String,
    order_id: String,
    total_price_rub: i64,
    receipt_uuid: String,
    receipt_url: String,
}

fn render_shop_receipt(data: ShopReceiptTemplateData) -> EmailTemplate {
    let product_name = escape_html(&data.product_name);
    let order_id = escape_html(&data.order_id);
    let receipt_uuid = escape_html(&data.receipt_uuid);
    let receipt_url = escape_html(&data.receipt_url);
    let total_price_rub = data.total_price_rub;

    EmailTemplate {
        subject: data.subject,
        html_body: format!(
            r#"<!doctype html>
<html>
<body style="margin:0;padding:24px;background:#f6f7fb;font-family:Arial,sans-serif;color:#1f2937">
  <div style="max-width:620px;margin:0 auto;background:#ffffff;border:1px solid #e5e7eb;padding:24px">
    <h1 style="margin:0 0 16px;font-size:22px;line-height:1.25;color:#111827">Спасибо за покупку</h1>
    <p style="margin:0 0 14px;font-size:15px;line-height:1.5">Мы выписали чек по твоему заказу на SvoCraft.</p>
    <table style="width:100%;border-collapse:collapse;margin:18px 0;font-size:14px">
      <tr>
        <td style="padding:8px 0;color:#6b7280">Покупка</td>
        <td style="padding:8px 0;text-align:right;color:#111827">{product_name}</td>
      </tr>
      <tr>
        <td style="padding:8px 0;color:#6b7280">Сумма</td>
        <td style="padding:8px 0;text-align:right;color:#111827">{total_price_rub} ₽</td>
      </tr>
      <tr>
        <td style="padding:8px 0;color:#6b7280">Заказ</td>
        <td style="padding:8px 0;text-align:right;color:#111827">{order_id}</td>
      </tr>
      <tr>
        <td style="padding:8px 0;color:#6b7280">Номер чека</td>
        <td style="padding:8px 0;text-align:right;color:#111827">{receipt_uuid}</td>
      </tr>
    </table>
    <p style="margin:0 0 12px;font-size:14px;line-height:1.5">Картинка чека приложена к письму и показана ниже.</p>
    <div style="margin:0 0 16px;text-align:center">
      <img src="__SVO_RECEIPT_IMAGE_SRC__" alt="Чек SvoCraft" style="max-width:100%;height:auto;border:1px solid #e5e7eb" />
    </div>
    <p style="margin:0 0 16px;font-size:14px;line-height:1.5">Если картинка не открылась, чек можно посмотреть по ссылке:</p>
    <p style="margin:0">
      <a href="{receipt_url}" style="color:#2563eb;text-decoration:none">Открыть чек</a>
    </p>
  </div>
</body>
</html>"#
        ),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
