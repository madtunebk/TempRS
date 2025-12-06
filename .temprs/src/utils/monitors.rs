pub fn detect_primary_monitor_xrandr() -> Option<(i32, i32, i32, i32)> {
    use std::process::Command;
    if let Ok(output) = Command::new("xrandr").arg("--query").output() {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                for line in s.lines() {
                    if line.contains(" connected primary ") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for part in parts {
                            if part.contains("+") && part.contains("x") {
                                if let Some((geom, _pos)) = part.split_once('+') {
                                    if let Some((w_str, h_str)) = geom.split_once('x') {
                                        if let (Ok(w), Ok(h)) =
                                            (w_str.parse::<i32>(), h_str.parse::<i32>())
                                        {
                                            let coords: Vec<&str> = part.split('+').collect();
                                            if coords.len() >= 3 {
                                                if let (Ok(x), Ok(y)) = (
                                                    coords[1].parse::<i32>(),
                                                    coords[2].parse::<i32>(),
                                                ) {
                                                    return Some((x, y, w, h));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for line in s.lines() {
                    if line.contains(" connected ") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for part in parts {
                            if part.contains("+") && part.contains("x") {
                                if let Some((geom, _)) = part.split_once('+') {
                                    if let Some((w_str, h_str)) = geom.split_once('x') {
                                        if let (Ok(w), Ok(h)) =
                                            (w_str.parse::<i32>(), h_str.parse::<i32>())
                                        {
                                            let coords: Vec<&str> = part.split('+').collect();
                                            if coords.len() >= 3 {
                                                if let (Ok(x), Ok(y)) = (
                                                    coords[1].parse::<i32>(),
                                                    coords[2].parse::<i32>(),
                                                ) {
                                                    return Some((x, y, w, h));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
