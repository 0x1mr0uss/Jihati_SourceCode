use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use colored::Colorize;
use local_ip_address::local_ip;
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

// هذه الهيكلة (Structs) تطابق تماماً البيانات القادمة من script.js
#[derive(Deserialize, Debug)]
struct Location {
    lat: f64,
    lng: f64,
}

#[derive(Deserialize, Debug)]
struct ReportPayload {
    category: String,
    description: String,
    location: Location,
}

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let logo = r#"
     ██╗██╗██╗  ██╗ █████╗ ████████╗██╗    ███████╗███████╗██████╗ ██╗   ██╗███████╗██████╗ 
     ██║██║██║  ██║██╔══██╗╚══██╔══╝██║    ██╔════╝██╔════╝██╔══██╗██║   ██║██╔════╝██╔══██╗
     ██║██║███████║███████║   ██║   ██║    ███████╗█████╗  ██████╔╝██║   ██║█████╗  ██████╔╝
██   ██║██║██╔══██║██╔══██║   ██║   ██║    ╚════██║██╔══╝  ██╔══██╗╚██╗ ██╔╝██╔══╝  ██╔══██╗
╚█████╔╝██║██║  ██║██║  ██║   ██║   ██║    ███████║███████╗██║  ██║ ╚████╔╝ ███████╗██║  ██║
 ╚════╝ ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝    ╚══════╝╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝
    "#;

    println!("{}", logo.bright_green().bold());

    let serve_dir = ServeDir::new("public");

    // قمنا بربط المسار /api/reports بالدالة التي ستعالج البلاغ
    let app = Router::new()
        .route("/api/reports", post(handle_report))
        .fallback_service(serve_dir);

    let port = 3000;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("🚀 Server is running!\n");
    println!("💻 Access locally via: {}", format!("http://localhost:{}", port).cyan());

    match local_ip() {
        Ok(my_local_ip) => {
            println!("📱 Access over WiFi via: {}", format!("http://{}:{}", my_local_ip, port).cyan());
        }
        Err(e) => {
            println!("{} {}", "⚠️ Could not automatically detect local IP:".yellow(), e);
        }
    }
    
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// الدالة التي تستقبل البلاغ وترسله إلى الإيميل عبر Resend
async fn handle_report(Json(payload): Json<ReportPayload>) -> impl IntoResponse {
    println!("{} {:?}", "📥 تم استلام بلاغ جديد:".bright_blue(), payload.category);

    //  ضع مفتاح الـ API الخاص بـ Resend هنا 
    let resend_api_key = "re_331HrTNv_wDVaP1CdXUYpXAWxtbmn8zyd"; 

    // تنسيق محتوى الإيميل بـ HTML
    let email_html = format!(
        r#"
        <div dir="rtl" style="font-family: Arial, sans-serif; padding: 20px; color: #333;">
            <h2 style="color: #0d5231;"> بلاغ جديد من منصة جهتي</h2>
            <div style="background: #f8fafc; padding: 15px; border-radius: 8px; border-right: 4px solid #1e8349;">
                <p><strong>نوع المشكل:</strong> {}</p>
                <p><strong>الوصف:</strong> {}</p>
                <p><strong>الإحداثيات:</strong> {}, {}</p>
            </div>
            <br>
            <a href="https://www.google.com/maps?q={},{}&z=17" 
               style="background: #1e8349; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px; font-weight: bold;">
               📍 عرض الموقع على خرائط جوجل
            </a>
        </div>
        "#,
        payload.category,
        payload.description,
        payload.location.lat,
        payload.location.lng,
        payload.location.lat,
        payload.location.lng
    );

    let client = reqwest::Client::new();
    
    // إرسال الطلب إلى Resend
    let res = client.post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", resend_api_key))
        .json(&serde_json::json!({
            "from": "onboarding@resend.dev", // إيميل الاختبار الافتراضي لـ Resend
            "to": "abdelyassin162@gmail.com",
            "subject": format!("بلاغ جديد: {}", payload.category),
            "html": email_html
        }))
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            println!("{}", "✅ تم إرسال الإيميل بنجاح!".bright_green());
            (StatusCode::OK, "تم الاستلام والإرسال").into_response()
        }
        Ok(response) => {
            let error_text = response.text().await.unwrap_or_default();
            println!("{} {}", " فشل إرسال الإيميل من Resend:".red(), error_text);
            (StatusCode::INTERNAL_SERVER_ERROR, "مشكلة في إرسال الإيميل").into_response()
        }
        Err(e) => {
            println!("{} {}", " خطأ في الاتصال بسيرفر الإيميل:".red(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, "خطأ في الخادم").into_response()
        }
    }
}