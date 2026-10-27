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

1. ✅ **已完成(2026-10-27)**:`survivors/i18n.rs` — 69 条 zh 目录(静态排序表 + 二分查找)、`t`/`tf`(支持 `{}` 顺序与 `{name}` 命名占位,UTF-8 安全)与 `tr!` 宏;调用点已迁移:menu、cards 品质名、char_select(字体/种族/职业)、hud(HUD 行、升级卡、暂停菜单、结算屏、效果描述)。
2. 语言来源(v1):`WOW_LANG`(`webenv::var`,网页即 `?lang=zhcn`)。自动探测数据 locale 留 v2——HUD 字体在 spawn 时选定,中途切语言会出豆腐块;探测需提前到 boot(探测点:`Fonts\FZLBJW.TTF` 只存在于 zhCN 数据)。
3. **Phase 2(未做)**:`ability_body` 的复合法术描述(Bolt/Nova/Ground/Chain 等 ~15 个分支)仍是英文,迁移方式与上面相同(逐分支 `tr!`);`data.rs` 的 zone/victory 文案(604 条字面量,按 key 建 zh 表)。
4. CJK 渲染:zhCN 数据自带 `fonts.MPQ`(26MB,含中文字体),渲染管线按客户端机制读字体;若实测有缺字形,优先查 `benilla-ui` 的 layout/justify 与 `benilla-formats` 的字体解析。

## 本机开发

- 工具链:rustup(1.98.1,由 `rust-toolchain.toml` 锁定),PATH 已配好;
- 原生运行:`make native`(数据走 `wow-data/Data`,当前 zhCN);
- 浏览器构建:`scripts/web-setup.sh`(一次性)→ `make wasm` → `make start`;
- 部署侧:Docker/隧道/账号体系见 `DOCKER-DEPLOY.md`,与代码开发互不影响。
