use std::process::{Command, exit};
use std::env;

fn main() {
    // Собираем аргументы, переданные этому экзешнику
    let args: Vec<String> = env::args().skip(1).collect();

    // Вызываем настоящий npx.cmd
    // Важно: здесь мы явно указываем .cmd, чтобы Windows нашла скрипт Node.js
    let mut child = Command::new("npx.cmd")
        .args(&args)
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("Failed to execute npx.cmd: {}", e);
            exit(1);
        });

    // Ждем завершения и возвращаем тот же код выхода
    let status = child.wait().unwrap_or_else(|e| {
        eprintln!("Failed to wait on npx.cmd: {}", e);
        exit(1);
    });

    exit(status.code().unwrap_or(1));
}