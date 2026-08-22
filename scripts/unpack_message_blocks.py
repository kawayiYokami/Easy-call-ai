#!/usr/bin/env python3
"""Unpack V4 message-store zstd blocks for inspection.

给定数据目录（或会话目录），流式解压全部 .jsonl.zstd 块到输出目录，
并对照 chat_metadata.sqlite 的 message_locator 做结束偏移对账。

用法：
    python scripts/unpack_message_blocks.py D:\\paitest\\data
    python scripts/unpack_message_blocks.py D:\\paitest\\data --conversation ac7ed70e-...
    python scripts/unpack_message_blocks.py D:\\paitest\\data\\chat\\conversations\\ac7ed70e-...
"""

from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path


def find_blocks(root: Path) -> list[tuple[str, Path]]:
    """返回 [(conversation_id, block_path)]。root 可以是数据根、conversations 目录或单会话目录。"""
    results: list[tuple[str, Path]] = []
    if (root / "blocks").is_dir():
        # 单会话目录：D:/.../conversations/<id>
        conversation_id = root.name
        for block in sorted((root / "blocks").glob("*.jsonl.zstd")):
            results.append((conversation_id, block))
        return results
    if (root / "conversations").is_dir():
        conv_root = root / "conversations"
    elif (root / "chat" / "conversations").is_dir():
        conv_root = root / "chat" / "conversations"
    else:
        return results
    for conv_dir in sorted(conv_root.iterdir()):
        if not conv_dir.is_dir():
            continue
        blocks_dir = conv_dir / "blocks"
        if not blocks_dir.is_dir():
            continue
        for block in sorted(blocks_dir.glob("*.jsonl.zstd")):
            results.append((conv_dir.name, block))
    return results


def unpack_block(block_path: Path, out_path: Path) -> tuple[int, int, int]:
    """流式解压全部 zstd 帧，返回 (压缩字节, 明文字节, 行数)。"""
    compressed = block_path.stat().st_size
    try:
        import zstandard
    except ImportError:
        sys.exit("缺少 zstandard 库，请先执行：pip install zstandard")
    plain_bytes = 0
    line_count = 0
    with open(block_path, "rb") as f:
        reader = zstandard.ZstdDecompressor().stream_reader(f)
        with open(out_path, "wb") as out:
            while True:
                chunk = reader.read(1 << 20)
                if not chunk:
                    break
                out.write(chunk)
                plain_bytes += len(chunk)
                line_count += chunk.count(b"\n")
    return compressed, plain_bytes, line_count


def reconcile_locator(data_root: Path, conversation_id: str, plain_bytes: int) -> str:
    """对照 chat_metadata.sqlite 的 message_locator，返回对账结果描述。"""
    # 从会话目录向上回溯找 chat_metadata.sqlite：支持数据根/单会话目录两种入口
    candidates = [
        data_root / "chat" / "chat_metadata.sqlite",
        data_root / "chat_metadata.sqlite",
        data_root / "state" / "state.sqlite",
    ]
    meta_db = next((p for p in candidates if p.is_file()), None)
    if meta_db is None:
        # 单会话目录入口：向上找 <data>/chat/chat_metadata.sqlite
        for parent in data_root.parents:
            probe = parent / "chat" / "chat_metadata.sqlite"
            if probe.is_file():
                meta_db = probe
                break
    if meta_db is None:
        return "对账跳过：未找到 chat_metadata.sqlite"
    try:
        conn = sqlite3.connect(f"file:{meta_db}?mode=ro", uri=True)
        cur = conn.cursor()
        last = cur.execute(
            "SELECT MAX(byte_offset + byte_len) FROM message_locator WHERE conversation_id=?",
            (conversation_id,),
        ).fetchone()
        conn.close()
    except sqlite3.Error as err:
        return f"对账失败：{err}"
    expected = last[0] if last and last[0] is not None else None
    if expected is None:
        return "对账跳过：该会话无 message_locator 记录"
    if expected == plain_bytes:
        return f"对账一致：locator 结束偏移 {expected} == 明文 {plain_bytes}"
    return f"对账不一致！locator 结束偏移 {expected} != 明文 {plain_bytes}"


def main() -> None:
    parser = argparse.ArgumentParser(description="解压 V4 消息存储 zstd 块并做 locator 对账")
    parser.add_argument("dir", help="数据根目录 / conversations 目录 / 单会话目录")
    parser.add_argument("--out", default=None, help="输出目录（默认：<dir 同级>/unpacked-blocks）")
    parser.add_argument("--conversation", default=None, help="只解压指定会话")
    args = parser.parse_args()

    root = Path(args.dir).resolve()
    if not root.is_dir():
        sys.exit(f"目录不存在：{root}")

    # 输出目录默认放在输入目录同级，避免污染数据目录
    out_root = Path(args.out).resolve() if args.out else root.parent / "unpacked-blocks"

    blocks = find_blocks(root)
    if args.conversation:
        blocks = [(cid, path) for cid, path in blocks if cid == args.conversation]
    if not blocks:
        sys.exit(f"未找到任何 .jsonl.zstd 块：{root}")

    total_compressed = 0
    total_plain = 0
    print(f"输出目录：{out_root}")
    for conversation_id, block_path in blocks:
        rel = block_path.relative_to(root) if block_path.is_relative_to(root) else block_path
        out_path = out_root / conversation_id / block_path.parent.name / block_path.name.replace(".zstd", "")
        out_path.parent.mkdir(parents=True, exist_ok=True)
        compressed, plain, lines = unpack_block(block_path, out_path)
        total_compressed += compressed
        total_plain += plain
        reconcile = reconcile_locator(root, conversation_id, plain)
        print(
            f"[{conversation_id[:12]}] {block_path.name}: 压缩 {compressed} -> 明文 {plain} 字节，{lines} 行 | {reconcile}"
        )
        print(f"    -> {out_path}")

    print(f"\n合计：{len(blocks)} 个块，压缩 {total_compressed} -> 明文 {total_plain} 字节")


if __name__ == "__main__":
    main()
