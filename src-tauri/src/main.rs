#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Some(pos) = args.iter().position(|a| a == "--context-menu-lock") {
        if let Some(path) = args.get(pos + 1) {
            let result = omnilock::shell_context::handle_context_menu_lock(path);
            match &result {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("{}", e),
            }
            return;
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--context-menu-unlock") {
        if let Some(path) = args.get(pos + 1) {
            let result = omnilock::shell_context::handle_context_menu_unlock(path);
            match &result {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("{}", e),
            }
            return;
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--lock-action") {
        if let Some(action) = args.get(pos + 1) {
            if let Some(path) = args.get(pos + 2) {
                let result = match action.as_str() {
                    "lock" => omnilock::shell_context::handle_context_menu_lock(path),
                    "unlock" => omnilock::shell_context::handle_context_menu_unlock(path),
                    _ => Err(format!("Unknown action: {}", action)),
                };
                match &result {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => eprintln!("{}", e),
                }
                return;
            }
        }
    }

    omnilock::run();
}
