# 本地化开发笔记(zhCN)

目标:为幸存者玩法的界面文案提供中文支持。本文记录文案分布、现状与建议路径。

## 文案的三层分布

| 层 | 来源 | 现状 |
|---|---|---|
| 幸存者玩法 UI(HUD/菜单/升级卡/结算) | **硬编码英文**,在 `crates/benilla-app/src/survivors/`(hud.rs / menu.rs / cards.rs / grade.rs 等) | 本地化主战场 |
| 魔兽世界内容(物品/技能/NPC/UI 框架) | 运行时读客户端数据(GlobalStrings.lua、DBC) | ✅ 已解决:`wow-data/Data` 已换成 zhCN 客户端(enUS 备份在 `wow-data/Data-enUS`) |
| 消息目录(DisplayError 机制) | `crates/benilla-ui/src/messages.rs` — 忠实复刻原版表格,**文本本身读自数据** | 无需改动,数据换 zhCN 即生效 |

## 关键机制

- `crates/benilla-ui/src/strings.rs`:`SStrPrintf` 模板填充——`GlobalStrings.lua` / `GlueStrings.lua` 的 `%s/%d/%N$s` 模板在运行时填参,**模板来自数据文件**,语言随数据走。
- 幸存者 UI 的字符串形如 `"Choose your power"`、带 `{}` 占位(在渲染时另行格式化),与上面的 `%` 模板体系**不同**,需要单独的 i18n 通道。

## 建议路径

1. 把 `survivors/*.rs` 的字面量抽成静态 catalog(仿 `benilla-ui/src/messages` 的表驱动风格),键为英文原文、值为译文,缺省回退英文;
2. 语言来源:从客户端数据的 locale 推导(zhCN 数据 → 中文),避免再造配置面;
3. CJK 渲染:zhCN 数据自带 `fonts.MPQ`(26MB,含中文字体),渲染管线按客户端机制读字体;若实测有缺字形,优先查 `benilla-ui` 的 layout/justify 与 `benilla-formats` 的字体解析。

## 本机开发

- 工具链:rustup(1.98.1,由 `rust-toolchain.toml` 锁定),PATH 已配好;
- 原生运行:`make native`(数据走 `wow-data/Data`,当前 zhCN);
- 浏览器构建:`scripts/web-setup.sh`(一次性)→ `make wasm` → `make start`;
- 部署侧:Docker/隧道/账号体系见 `DOCKER-DEPLOY.md`,与代码开发互不影响。
