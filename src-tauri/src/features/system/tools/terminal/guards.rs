fn terminal_is_powershell_encoded_command(command: &str) -> bool {
    let tokens = terminal_tokenize(command);
    if tokens.is_empty() {
        return false;
    }

    let mut saw_powershell = false;
    let mut saw_encoded_flag = false;
    for token in tokens {
        let unquoted = terminal_unquote_token(&token);
        let exe_name = unquoted
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(unquoted.as_str());
        let lower = exe_name.to_ascii_lowercase();
        let lower_full = unquoted.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe"
        ) {
            saw_powershell = true;
        }
        if matches!(lower_full.as_str(), "-encodedcommand" | "-enc" | "-e")
            || lower_full.starts_with("-encodedcommand:")
            || lower_full.starts_with("-enc:")
            || lower_full.starts_with("-e:")
        {
            saw_encoded_flag = true;
        }
    }
    saw_powershell && saw_encoded_flag
}

fn terminal_git_dangerous_block_reason(command: &str) -> Option<&'static str> {
    for simple in terminal_split_simple_commands(command) {
        let Some(first) = simple.argv.first() else {
            continue;
        };
        if terminal_unquote_token(first).to_ascii_lowercase() != "git" {
            continue;
        }
        // 跳过 git 全局选项及其参数（-C <path>、-c <key>=<val> 等），
        // 之后第一个非选项 token 才是子命令；否则 -C 会占位导致 push 拦截失效
        let mut sub_index = 1usize;
        while sub_index < simple.argv.len() {
            let token = terminal_unquote_token(&simple.argv[sub_index]);
            if !token.starts_with('-') {
                break;
            }
            // 带参数的全局选项：跳过其参数
            let takes_value = matches!(
                token.as_str(),
                "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" | "--exec-path"
                    | "--shallow-file" | "--config-env"
            ) || token.starts_with("--git-dir=")
                || token.starts_with("--work-tree=")
                || token.starts_with("--namespace=")
                || token.starts_with("--exec-path=")
                || token.starts_with("-c=");
            sub_index += 1;
            if takes_value && sub_index < simple.argv.len() {
                sub_index += 1;
            }
        }
        let second = simple
            .argv
            .get(sub_index)
            .map(|item| terminal_unquote_token(item).to_ascii_lowercase())
            .unwrap_or_default();
        // 子命令之后的参数（跳过全局选项后定位）
        let sub_args = simple.argv.iter().skip(sub_index + 1);
        let has_force_flag = sub_args.clone().any(|item| {
            let token = terminal_unquote_token(item).to_ascii_lowercase();
            matches!(token.as_str(), "-f" | "--force")
        });

        match second.as_str() {
            // push 会改写远端状态且影响共享仓库，一律拦截（用户策略：push 须经确认）
            "push" => {
                return Some("git push is especially dangerous and is blocked");
            }
            "pull" if has_force_flag => {
                return Some("git pull --force/-f is especially dangerous and is blocked");
            }
            "reset" => return Some("git reset is blocked"),
            "clean" => return Some("git clean is blocked"),
            // 强删分支（-D）会丢弃整条分支；--delete --force 与 -d -f 是等价组合，
            // -d 单独是安全删除不拦（注意 -D 不能转小写，否则与 -d 混淆）
            "branch" => {
                let raw_args: Vec<String> = sub_args
                    .clone()
                    .map(|item| terminal_unquote_token(item))
                    .collect();
                let delete = raw_args.iter().any(|t| {
                    matches!(t.as_str(), "-d" | "--delete" | "-D")
                });
                let force = raw_args.iter().any(|t| {
                    matches!(t.as_str(), "-f" | "--force" | "-D")
                });
                if delete && force {
                    return Some("git branch -D is especially dangerous and is blocked");
                }
            }
            // 清空/删除储藏不可恢复
            "stash" => {
                let third = sub_args
                    .clone()
                    .next()
                    .map(|item| terminal_unquote_token(item).to_ascii_lowercase())
                    .unwrap_or_default();
                if matches!(third.as_str(), "clear" | "drop") {
                    return Some("git stash clear/drop is especially dangerous and is blocked");
                }
            }
            // rebase 改写提交历史，正常人不会主动用
            "rebase" => return Some("git rebase is especially dangerous and is blocked"),
            // 改远端目标影响后续 push 落点；rm 是 remove 的官方别名
            "remote" => {
                let third = sub_args
                    .clone()
                    .next()
                    .map(|item| terminal_unquote_token(item).to_ascii_lowercase())
                    .unwrap_or_default();
                if matches!(third.as_str(), "remove" | "rm" | "set-url") {
                    return Some("git remote remove/set-url is especially dangerous and is blocked");
                }
            }
            _ => {}
        }
    }
    None
}

fn terminal_command_block_reason(command: &str) -> Option<&'static str> {
    if terminal_is_powershell_encoded_command(command) {
        return Some("encoded command is blocked");
    }
    if let Some(reason) = terminal_git_dangerous_block_reason(command) {
        return Some(reason);
    }
    let lower = command.to_ascii_lowercase();
    if lower.contains("invoke-expression") || lower.contains("iex ") || lower.contains("iex(") {
        return Some("Invoke-Expression/iex is blocked");
    }
    if lower.contains("start-process")
        && (lower.contains("powershell")
            || lower.contains("pwsh")
            || lower.contains("cmd.exe")
            || lower.contains("/bin/sh")
            || lower.contains("/bin/bash"))
    {
        return Some("spawning nested shells is blocked");
    }
    None
}

fn terminal_decode_with_encoding(
    bytes: &[u8],
    encoding: &'static encoding_rs::Encoding,
) -> Option<String> {
    let (decoded, _, had_errors) = encoding.decode(bytes);
    if had_errors {
        return None;
    }
    Some(decoded.into_owned())
}

#[cfg(target_os = "windows")]
fn terminal_windows_system_encoding() -> Option<&'static encoding_rs::Encoding> {
    use windows_sys::Win32::Globalization::GetACP;

    let label = match unsafe { GetACP() } {
        936 => b"gbk".as_slice(),
        65001 => b"utf-8".as_slice(),
        1250 => b"windows-1250".as_slice(),
        1251 => b"windows-1251".as_slice(),
        1252 => b"windows-1252".as_slice(),
        1253 => b"windows-1253".as_slice(),
        1254 => b"windows-1254".as_slice(),
        1255 => b"windows-1255".as_slice(),
        1256 => b"windows-1256".as_slice(),
        1257 => b"windows-1257".as_slice(),
        1258 => b"windows-1258".as_slice(),
        874 => b"windows-874".as_slice(),
        932 => b"shift_jis".as_slice(),
        949 => b"euc-kr".as_slice(),
        950 => b"big5".as_slice(),
        866 => b"ibm866".as_slice(),
        437 => b"ibm437".as_slice(),
        850 => b"ibm850".as_slice(),
        _ => return None,
    };
    encoding_rs::Encoding::for_label(label)
}

fn terminal_detect_output_encoding(bytes: &[u8]) -> Option<&'static encoding_rs::Encoding> {
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(bytes, true);
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
    Some(encoding)
}

fn terminal_decode_output_bytes(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(encoding) = terminal_windows_system_encoding() {
            if let Some(decoded) = terminal_decode_with_encoding(bytes, encoding) {
                return decoded;
            }
        }
    }

    if let Some(encoding) = terminal_detect_output_encoding(bytes) {
        if let Some(decoded) = terminal_decode_with_encoding(bytes, encoding) {
            return decoded;
        }
    }

    #[cfg(target_os = "windows")]
    if let Some(decoded) = terminal_decode_with_encoding(bytes, encoding_rs::GBK) {
        return decoded;
    }

    String::from_utf8_lossy(bytes).to_string()
}

fn truncate_terminal_output(bytes: &[u8]) -> (String, bool) {
    if bytes.len() <= TERMINAL_MAX_OUTPUT_BYTES {
        return (terminal_decode_output_bytes(bytes), false);
    }
    (
        terminal_decode_output_bytes(&bytes[..TERMINAL_MAX_OUTPUT_BYTES]),
        true,
    )
}

fn terminal_is_timeout_error(err: &str) -> bool {
    err.to_ascii_lowercase().contains("timed out after")
}

#[cfg(test)]
mod terminal_output_decode_tests {
    use super::*;

    #[test]
    fn decode_utf8_output_should_keep_utf8_text() {
        assert_eq!(terminal_decode_output_bytes("中文".as_bytes()), "中文");
    }

    #[test]
    fn oversized_terminal_output_should_be_marked_truncated() {
        let bytes = vec![b'x'; TERMINAL_MAX_OUTPUT_BYTES + 1];
        let (text, truncated) = truncate_terminal_output(&bytes);
        assert!(truncated);
        assert_eq!(text.len(), TERMINAL_MAX_OUTPUT_BYTES);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn decode_windows_gbk_output_should_fallback_to_gbk() {
        let bytes = [0xd6, 0xd0, 0xce, 0xc4];
        assert_eq!(terminal_decode_output_bytes(&bytes), "中文");
    }

    #[test]
    fn detect_windows_1252_punctuation_should_not_become_garbled() {
        let bytes = [0x93, b'H', b'e', b'l', b'l', b'o', 0x94];
        assert_eq!(terminal_decode_output_bytes(&bytes), "“Hello”");
    }

    #[test]
    fn git_push_should_be_blocked_by_local_rule() {
        // 普通 push 同样拦截（用户策略：push 会改写共享远端，须经确认）
        assert_eq!(
            terminal_command_block_reason("git push origin main"),
            Some("git push is especially dangerous and is blocked")
        );
        assert_eq!(
            terminal_command_block_reason("git push --force origin main"),
            Some("git push is especially dangerous and is blocked")
        );
        // 带 -f 短标志也一样
        assert_eq!(
            terminal_command_block_reason("git push -f origin main"),
            Some("git push is especially dangerous and is blocked")
        );
    }

    #[test]
    fn git_push_with_global_options_should_be_blocked() {
        // -C <path> 全局选项：跳过选项及其参数后再识别 push 子命令
        assert_eq!(
            terminal_command_block_reason("git -C /repo push origin main"),
            Some("git push is especially dangerous and is blocked")
        );
        // 多个全局选项连用
        assert_eq!(
            terminal_command_block_reason("git -C /repo -c user.name=test push origin main"),
            Some("git push is especially dangerous and is blocked")
        );
        // 非 git 命令不受影响
        assert_eq!(terminal_command_block_reason("git -C /repo status"), None);
    }

    #[test]
    fn git_branch_force_delete_should_be_blocked() {
        assert_eq!(
            terminal_command_block_reason("git branch -D old-feature"),
            Some("git branch -D is especially dangerous and is blocked")
        );
        // --delete --force 与 -d -f 是 -D 的等价组合，同样拦截
        assert_eq!(
            terminal_command_block_reason("git branch --delete --force old-feature"),
            Some("git branch -D is especially dangerous and is blocked")
        );
        assert_eq!(
            terminal_command_block_reason("git branch -d -f old-feature"),
            Some("git branch -D is especially dangerous and is blocked")
        );
        // 安全删除 -d 不拦
        assert_eq!(terminal_command_block_reason("git branch -d old-feature"), None);
        // 单独 --delete 或单独 --force 不拦（不是强删组合）
        assert_eq!(terminal_command_block_reason("git branch --delete old-feature"), None);
        assert_eq!(terminal_command_block_reason("git branch -f new-branch"), None);
        // 普通 branch 列表/新建不拦
        assert_eq!(terminal_command_block_reason("git branch"), None);
        assert_eq!(terminal_command_block_reason("git branch new-feature"), None);
    }

    #[test]
    fn git_stash_clear_drop_should_be_blocked() {
        assert_eq!(
            terminal_command_block_reason("git stash clear"),
            Some("git stash clear/drop is especially dangerous and is blocked")
        );
        assert_eq!(
            terminal_command_block_reason("git stash drop stash@{0}"),
            Some("git stash clear/drop is especially dangerous and is blocked")
        );
        // 普通 stash 操作不拦
        assert_eq!(terminal_command_block_reason("git stash"), None);
        assert_eq!(terminal_command_block_reason("git stash apply"), None);
    }

    #[test]
    fn git_rebase_should_be_blocked() {
        assert_eq!(
            terminal_command_block_reason("git rebase main"),
            Some("git rebase is especially dangerous and is blocked")
        );
        assert_eq!(
            terminal_command_block_reason("git rebase -i HEAD~3"),
            Some("git rebase is especially dangerous and is blocked")
        );
    }

    #[test]
    fn git_remote_remove_set_url_should_be_blocked() {
        assert_eq!(
            terminal_command_block_reason("git remote remove origin"),
            Some("git remote remove/set-url is especially dangerous and is blocked")
        );
        // rm 是 remove 的官方别名，同样拦截
        assert_eq!(
            terminal_command_block_reason("git remote rm origin"),
            Some("git remote remove/set-url is especially dangerous and is blocked")
        );
        assert_eq!(
            terminal_command_block_reason("git remote set-url origin https://example.com/repo.git"),
            Some("git remote remove/set-url is especially dangerous and is blocked")
        );
        // 查看远端不拦
        assert_eq!(terminal_command_block_reason("git remote -v"), None);
        assert_eq!(terminal_command_block_reason("git remote add upstream https://example.com/up.git"), None);
    }

    #[test]
    fn git_pull_should_not_be_blocked_by_local_rule() {
        assert_eq!(terminal_command_block_reason("git pull origin main"), None);
    }

    #[test]
    fn git_commit_should_not_be_blocked_by_local_rule() {
        assert_eq!(terminal_command_block_reason("git commit -m \"msg\""), None);
    }
}
