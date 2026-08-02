# 社区治理系统（Moderation）架构设计

> 阶段：第十三阶段 · 设计文档（步骤 ①）
> 分支：feature/moderation
> 状态：**待确认**（确认设计符合现有架构后再开始实现）

---

## 1. 现有架构检查结论

### 1.1 Monorepo 结构（与设计约束）

| 目录 | 说明 | 治理系统复用点 |
| --- | --- | --- |
| `apps/api` | Rust Axum + SQLx + PostgreSQL + Redis | 全部后端逻辑 |
| `apps/web` | Next.js App Router + TanStack Query + react-hook-form + zod | 全部前端页面 |
| `packages/types` | 前后端共享 API 类型 | 新增治理相关类型 |
| `packages/ui` | shadcn 风格组件（Button/Input/Textarea/Select/Alert/Badge/Avatar/Label） | 复用，**无 Dialog 组件**（举报/二次确认用内联表单 + confirm） |
| `docs/` | architecture / deployment / ops | 新增 `docs/moderation/` |

### 1.2 后端分层（必须遵循）

```
src/
├── models/        # 领域类型 + RBAC 常量（permission 常量集中在 rbac.rs）
├── repositories/  # 纯 SQL 访问，返回模型类型
├── services/      # 业务逻辑 + 权限校验（require/require_owner_or_any 模式）
├── routes/        # axum Router，/admin/* 使用 AuthorizationLayer + require_permission
├── middleware/    # Bearer JWT 认证 + CSRF origin 校验
├── realtime/      # RealtimeHub（内存广播）+ RealtimeBus（Redis Pub/Sub，channel: realtime:user-events）
├── events/        # 领域事件 → NotificationService.handle_event()
├── state.rs       # AppState 组合根（所有 service 在此组装）
├── error.rs       # 统一错误：{ "error": { "code", "message" } }
└── migrations/    # sqlx 时间戳迁移（YYYYMMDDHHMMSS_name.up/down.sql）
```

### 1.3 关键现有实现与复用点

| 系统 | 现状 | 复用方式 |
| --- | --- | --- |
| **认证** | JWT access + refresh；`AuthorizationService` 校验 `status=='active'` + `auth_version` + `role` 与 claims 匹配；Redis 缓存快照 `authz:user:{id}`；管理员改状态时 bump auth_version + 撤销 refresh token | 封禁 = `users.status` 变更 + auth_version bump + refresh 撤销（**现有机制已满足**，无需新逻辑） |
| **RBAC** | `roles`（user=10 / moderator=20 / administrator=30 / super_administrator=40，priority 越大越高）；`permissions` + `role_permissions` 由 migration 播种；`AuthenticatedPrincipal::has_permission` 用 HashSet | 新增 `moderation.*` 权限 + `senior_moderator` 角色（priority 25），沿用现有 `require()` 双校验（admin.access + 具体权限） |
| **Topics** | `status: published/hidden/deleted`；`is_pinned/is_featured`；`moderate()` 仅支持 pin/feature | 扩展 `is_locked/is_sensitive/restrict_interactions` 字段 |
| **Comments** | `status: published/deleted`；软删除；两级嵌套（楼中楼，DB trigger 限制）；`delete/restore` | 扩展 `hidden` 状态 + `is_collapsed/is_sensitive/replies_locked` 字段 |
| **Reports** | `reports` 表：status `open/reviewing/resolved/rejected`；`POST /reports`（report.create）；`GET/PATCH /admin/reports`（report.manage）；无优先级/无合并/无历史 | **扩展而非重写**：加 `priority/duplicate_of/case_id` 列、`duplicate/cancelled` 状态、`report_events` 历史表 |
| **Notification** | 单表 + `type` CHECK 约束 + jsonb metadata；`NotificationService.handle_event/create_inbox`；WebSocket 推送 `notification.created` | 扩展 type CHECK 增加治理类型；新增 `dedup_key` 部分唯一索引保证幂等 |
| **Realtime** | `RealtimeBus.publish_to_user(user_id, type, data)` → Redis Pub/Sub → Hub；前端 `realtime-provider` 处理 `notification.created` | 治理事件复用同一通道（按用户推送）；前端扩展事件类型 |
| **Admin Panel** | `/admin/*` 全部挂在 `admin.access` + 具体权限下；`admin_logs` 表记录操作日志（action/target/metadata/ip/ua） | 治理操作统一写入 `admin_logs`（**不另建冲突日志**），另建 `moderation_actions` 作为业务历史可查询 |
| **Search** | PG FTS；查询已硬过滤 `status='published'`（topic/comment/user） | 隐藏/删除内容天然不进公开搜索；仅需确认 comment 查询过滤 `hidden` |
| **SEO** | `sitemap.ts` / `rss.xml` / JSON-LD 全部通过**公开 API** 取数 | 公开 API 过滤即 SEO 过滤；治理状态无需改前端 SEO 代码 |
| **限流** | 手动 Redis `incr + expire` 模式（search/comment/reaction 已用） | 举报/自动审核限流沿用此模式 |
| **测试** | 单元测试 `#[cfg(test)]` + 集成测试 `tests/*.rs`（读 `DATABASE_URL`，缺失则跳过） | 新增 `tests/moderation.rs` |
| **监控** | **无 Prometheus 代码**（Phase 12 部署文档无 metrics 配置） | 新增零依赖 `/metrics` 文本端点（手工 Prometheus 文本格式 + std 原子计数） |

### 1.4 与提示词的差异及兼容方案

| 提示词建议 | 现有架构事实 | 兼容方案 |
| --- | --- | --- |
| 举报状态 Pending/… | 现有 `open/reviewing/resolved/rejected` | 保留 `open`（语义= pending），**新增** `duplicate`、`cancelled`；文档中 open≡pending |
| 建议角色含 Senior Moderator | 无此角色 | 新增 `senior_moderator`（priority 25），migration 播种 |
| 建议路径 `/api/moderation/...` | 现有公开路径无 `/api` 前缀（`/reports`），管理路径为 `/admin/...` | 用户端：`/reports/me`、`/moderation/sanctions/me`、`/moderation/appeals/*`；管理端：`/admin/moderation/*` |
| 建议 `sanction_restrictions` 表 | 无 | 用 `user_sanctions.restrictions text[]` 列（避免过度拆表，文档说明） |
| Prometheus/Grafana | 仓库无 metrics 端点 | 新增零依赖 `/metrics`；Grafana Dashboard 以文档形式提供 |
| Admin 面板含 moderator 访问 | `RequireAdmin` 仅放行 admin/super_admin | 新增 `RequireStaff`（放行 moderator+），服务端权限仍按粒度校验 |

---

## 2. 总体设计

### 2.1 核心概念

- **Report（举报）**：用户对 topic/comment/user 的单一举报，含原因、说明、优先级。
- **Moderation Case（审核案例）**：对**同一目标**的治理单元。一个 open/reviewing 的 case 唯一对应一个目标（部分唯一索引保证）。首次举报或自动审核命中时自动建 case，多份举报关联到同一 case（相关举报）。
- **Moderation Action（治理动作）**：每次内容状态变更/处罚/恢复记录一行，含前后状态、原因、执行人、关联 case/report。
- **Content Snapshot（内容快照）**：治理时保存目标内容副本（隐私：仅保存治理所需字段，有保留期）。
- **Sanction（处罚）**：对用户的警告/禁言/封禁等，可叠加，可撤销，可过期。
- **Appeal（申诉）**：用户对处罚或内容处置的申诉，单次处罚限制申诉次数。

### 2.2 数据流（端到端）

```
[用户] 举报 topic/comment/user
  → POST /reports（限流 5/min + 防自举报 + 防重复）
  → reports 行 + report_events(created)
  → 若无 open case：创建 moderation_cases（target 维度）
  → reports.case_id = case.id
  → 通知审核员（moderation_inbox）+ 治理统计 + [可选] 用户通知(report_submitted)

[审核员] 审核队列
  → GET /admin/moderation/cases?status=open&priority=high...
  → 领取/释放/转交 case（状态校验防并发覆盖）
  → 详情：内容快照 + 相关举报 + 作者违规史 + 自动审核命中 + 操作历史 + 内部备注

[处理] 接受 → 内容动作（hide/delete/lock/...）→ 可选处罚（warn/mute/suspend/ban）
  → moderation_actions + admin_logs + content_snapshots + user_sanctions
  → 关闭 case（status=closed）+ 关联 reports 标记 resolved
  → 通知内容作者（content_hidden/...）+ 处罚通知用户（user_warned/...）

[申诉] 用户对处罚/内容处置申诉
  → POST /moderation/sanctions/{id}/appeals（限次数）
  → appeals + appeal_events → 审核员处理（非原执行人）
  → approved → 自动恢复内容/解除处罚 + 通知；rejected → 必须填原因 + 通知
```

### 2.3 状态机

**Report**（扩展现有表）：

```
open ──领取──▶ reviewing ──接受(处理)──▶ resolved
  │              │  └────拒绝──────────▶ rejected
  │              └────标记重复──────────▶ duplicate ──duplicate_of→ 目标报告
  └────取消────────────────────────────▶ cancelled
```
转移校验：`WHERE id=$1 AND status IN ('open','reviewing')`（幂等，防并发覆盖）。终态（resolved/rejected/duplicate/cancelled）不可再转移。

**Moderation Case**：

```
open ──领取──▶ reviewing ──关闭──▶ closed
  │              │  └─重新打开（内容误操作恢复后重新开启）─▶ open
  └────直接关闭（驳回举报）────────▶ closed
```
`closed` 后同目标可再开新 case。部分唯一索引 `(target_type, target_id) WHERE status IN ('open','reviewing')` 防止同目标并发多 case。

**Content（Topic/Comment）**：

```
published ──hide──▶ hidden ──restore──▶ published
published ──delete──▶ deleted（软删，deleted_at）──restore(仅comment，topic 恢复为 published)──▶ published
published ──lock(仅topic)──▶ published+locked（可展示不可评论）
published ──collapse(仅comment)──▶ published+collapsed（可展示，前端折叠）
任何状态 ──mark_sensitive──▶ +is_sensitive（标记徽标）
```

**Sanction**：

```
scheduled（未来开始）──▶ active ──到期（后台任务）──▶ expired
                     └─撤销（更高权限）──▶ revoked
active ──撤销──▶ revoked
```
叠加语义：同一用户多处罚并存，**权限限制取并集**；任意 active 的 suspension/ban 即整体禁入。

**Appeal**：

```
pending ──领取──▶ reviewing ──通过──▶ approved（触发自动恢复/解禁）
                    └─拒绝（必须填原因）──▶ rejected
pending ──用户取消──▶ cancelled
```
限制：同一 sanction 最多 2 次申诉；终态不可重复处理（`WHERE status IN ('pending','reviewing')`）。

### 2.4 内容可见性（角色 × 状态）

| 状态 | Guest/User（作者除外） | 内容作者 | Moderator+ | 说明 |
| --- | --- | --- | --- | --- |
| published | ✅ 公开 | ✅ | ✅ | 进搜索/SEO |
| hidden | ❌ 404 | ✅ 自己可见 | ✅ | 不进公开搜索/SEO |
| collapsed（comment） | ✅ 可见（折叠态） | ✅ | ✅ | 仍可搜索 |
| deleted | ❌ | ❌（提示已删除） | ✅ 管理后台 | 不进搜索/SEO/RSS/JSON-LD |
| locked（topic） | ✅ 展示，禁评论 | ✅ | ✅ | 仍可搜索 |
| under review（case 维度） | 内容保持原可见性，等待人工 | — | 队列可见 | 规则动作 queue 不改变内容状态；hide 才隐藏 |

服务端兜底：`TopicService::get_public / list_public`、`CommentService::list_for_topic` 均只返回 published；举报/治理接口校验作者身份后才返回 hidden 内容。

### 2.5 权限矩阵

新权限（全部服务端校验，沿用 `require()` = admin.access + 具体权限 的双校验模式）：

| 权限 | moderator | senior_moderator | administrator | super_administrator |
| --- | :-: | :-: | :-: | :-: |
| moderation.report.read（队列查看） | ✅ | ✅ | ✅ | ✅ |
| moderation.report.review（处理举报） | ✅ | ✅ | ✅ | ✅ |
| moderation.report.assign（领取/转交） | — | ✅ | ✅ | ✅ |
| moderation.content.hide / restore | ✅ | ✅ | ✅ | ✅ |
| moderation.content.delete | — | ✅ | ✅ | ✅ |
| moderation.topic.lock | ✅ | ✅ | ✅ | ✅ |
| moderation.topic.move | — | ✅ | ✅ | ✅ |
| moderation.user.warn / mute | ✅ | ✅ | ✅ | ✅ |
| moderation.user.suspend（临时封禁） | — | ✅ | ✅ | ✅ |
| moderation.user.ban（永久封禁） | — | — | ✅ | ✅ |
| moderation.sanction.revoke | — | ✅（禁解除永久封禁） | ✅ | ✅ |
| moderation.appeal.read | ✅ | ✅ | ✅ | ✅ |
| moderation.appeal.review | — | ✅ | ✅ | ✅ |
| moderation.rule.manage（规则/敏感词/域名） | — | — | ✅ | ✅ |
| moderation.audit.read | — | — | ✅ | ✅ |
| moderation.metrics.read | ✅ | ✅ | ✅ | ✅ |

**硬性规则**（服务层强制，不依赖前端）：
- 用户不能举报自己发布的内容（topic/comment author_id、user 目标 = 自己）。
- 审核员不能处理自己的举报/申诉（report.reporter_id / appeal.user_id ≠ handler）。
- 处罚目标 role_priority ≥ 执行人 priority 时拒绝（复用现有 `lock_user` + priority 比较模式，`>=` 禁止同级处罚）。
- 不能处罚自己；不能处罚 super_administrator（最后一名保护沿用现有逻辑）。
- 解除处罚：只允许解除**低于自己优先级**用户的处罚；永久封禁的解除仅 super_administrator 可执行且需二次确认字段。
- 批量操作逐项校验，部分越权时返回逐项结果（`{succeeded: [], failed: [{id, code}]}`），已处理项跳过（幂等）。

### 2.6 并发与一致性策略

| 场景 | 策略 |
| --- | --- |
| 举报领取 | `UPDATE moderation_cases SET assignee_id=$1, status='reviewing' WHERE id=$2 AND status='open' AND assignee_id IS NULL`，rows=0 → 冲突 |
| 举报状态流转 | `WHERE status IN ('open','reviewing')` 状态守卫，终态幂等 |
| 内容动作 | `SELECT ... FOR UPDATE` 锁定目标行（沿用 `lock_user` 模式）+ 事务内写 action/log/snapshot |
| 处罚/申诉 | 事务 + 状态守卫；申诉通过后的自动恢复与解除处罚在同一事务 |
| 通知幂等 | `notifications` 增加 `dedup_key` 部分唯一索引 `(user_id, type, (metadata->>'dedup_key'))` |
| 批量操作 | 逐项独立事务守卫，失败不整体回滚（记录逐项结果） |
| 自动审核 | 命中记录与内容创建同事务；规则缓存 Redis 60s |

### 2.7 数据保留策略

- **内容快照**：保留 90 天（后台任务清理），仅存治理所需字段（title/content/summary/status），不存完整用户资料。
- **举报/案例/处罚/申诉**：永久保留（合规审计需要）。
- **自动审核命中**：`content_snippet` 截断 500 字符，保留 180 天。
- **审核日志（admin_logs）**：永久保留。
- 敏感字段（IP/UA）只进 `admin_logs`，不出现在业务 API 响应。

---

## 3. 数据表设计（Migration：`20260812000000_moderation`）

全部沿用现有命名规范（snake_case、uuid 主键、set_updated_at trigger、CHECK 约束、时间戳迁移）。

### 3.1 表清单与关系

```
users ─┬─< reports（扩展：+priority, +duplicate_of→reports, +case_id）
       ├─< moderation_cases（target_type/target_id 部分唯一）
       ├─< moderation_actions（before/after jsonb、reason、operator）
       ├─< content_snapshots（治理快照，可清理）
       ├─< user_sanctions（restrictions text[]、期限、状态）
       ├─< appeals（处罚或内容申诉）
       ├─< moderation_notes（case 内部备注）
       └─< moderation_rule_hits

moderation_cases ─┬─< reports.case_id
                  ├─< moderation_actions.case_id
                  ├─< user_sanctions.case_id
                  └─< moderation_notes.case_id

user_sanctions ─< appeals.sanction_id（appeals 也可指向内容: content_type/content_id）
moderation_rules ─< moderation_rule_hits.rule_id
```

### 3.2 新建/扩展表（要点）

| 表 | 关键字段 | 约束/索引 |
| --- | --- | --- |
| `reports`（扩展） | 新增 `priority varchar(16) NOT NULL DEFAULT 'normal'`（low/normal/high/urgent）、`duplicate_of uuid REFERENCES reports(id)`、`case_id uuid REFERENCES moderation_cases(id)`、`risk_score int`、`cancelled_at` | status CHECK 增加 `duplicate/cancelled`；`reports_target_open_idx (target_type,target_id) WHERE status IN ('open','reviewing')`；`reports_case_idx (case_id)` |
| `report_events` | report_id、actor_type（reporter/system/moderator）、action（created/assigned/reviewing/resolved/rejected/duplicated/cancelled/note）、note、created_at | 索引 `(report_id, created_at)` |
| `moderation_cases` | target_type（topic/comment/user）、target_id、status（open/reviewing/closed）、priority、risk_score、source（report/auto/manual）、assignee_id、opened_by、closed_at | **部分唯一** `(target_type,target_id) WHERE status IN ('open','reviewing')`；索引 `(status, priority, created_at DESC)`、`(assignee_id)` |
| `moderation_actions` | case_id、action（hide/restore/delete/lock/unlock/pin/unpin/move_category/mark_sensitive/restrict_interactions/collapse/...）、target_type/target_id、before_status/after_status、reason、operator_id、report_id、sanction_id、created_at | 索引 `(case_id)`、`(target_type,target_id)`、`(operator_id, created_at DESC)` |
| `content_snapshots` | case_id、target_type/target_id、title、content、summary、status、reason、created_by、created_at | 索引 `(case_id)`、`(target_type,target_id,created_at DESC)` |
| `user_sanctions` | user_id、sanction_type（warning/content_restriction/mute/suspension/ban）、reason、user_visible_reason、internal_note、restrictions text[]（no_topics/no_comments/no_reports/no_uploads）、starts_at、ends_at、is_permanent、status（scheduled/active/expired/revoked）、issued_by、case_id、report_id、related_content_type/id、revoked_by/at/reason | 索引 `(user_id, status)`、`(status, ends_at)`、`(issued_by)`；CHECK `(is_permanent AND ends_at IS NULL) OR (NOT is_permanent AND ends_at IS NOT NULL)` |
| `appeals` | user_id、appeal_type（sanction/content）、sanction_id、content_type/content_id、reason、details、evidence jsonb（上传 id 数组）、status（pending/reviewing/approved/rejected/cancelled）、reviewer_id、review_note、reviewed_at | CHECK 二选一目标；索引 `(status, created_at)`、`(user_id, created_at)`、`(sanction_id)` |
| `appeal_events` | appeal_id、actor_type、action（submitted/reviewing/approved/rejected/cancelled）、note | 索引 `(appeal_id, created_at)` |
| `moderation_rules` | name、rule_type（keyword/url_domain/rate/duplicate/new_user/high_frequency）、target_type、priority、enabled、risk_score、action（allow/flag/queue/collapse/hide/reject/rate_limit）、config jsonb（关键词/阈值/窗口/域名）、created_by、hit_count | 索引 `(enabled, priority)` |
| `moderation_rule_hits` | rule_id、target_type/id、user_id、content_snippet（≤500）、risk_score、action、created_at | 索引 `(rule_id, created_at DESC)`、`(target_type,target_id)` |
| `moderation_notes` | case_id、author_id、note（内部，≤2000）、created_at | 索引 `(case_id, created_at)` |
| `topics`（扩展） | `is_locked boolean NOT NULL DEFAULT false`、`is_sensitive boolean NOT NULL DEFAULT false`、`restrict_interactions boolean NOT NULL DEFAULT false` | 无破坏性变更 |
| `comments`（扩展） | status CHECK 增加 `hidden`；`is_collapsed boolean`、`is_sensitive boolean`、`replies_locked boolean` | 软删约束同步放宽 |
| `notifications`（扩展） | type CHECK 增加治理类型；`metadata->>'dedup_key'` | **部分唯一索引** `(user_id, type, (metadata->>'dedup_key')) WHERE metadata->>'dedup_key' IS NOT NULL` |

### 3.3 后台任务（`state.rs` 内 tokio::spawn 周期任务，每分钟）

1. **处罚过期**：`active & !permanent & ends_at <= now()` → `expired`；若为 suspension 且无其他 active ban/suspension → `users.status` 恢复 active（auth_version 保持已 bump 的旧值，用户需重新登录，文档明确）。
2. **处罚到期提醒**：ends_at 在 24h 内 → 发送 `sanction_expiring` 通知（dedup_key 防重复）。
3. **快照清理**：`content_snapshots` 超 90 天删除。
4. **规则命中清理**：超 180 天删除。

### 3.4 封禁与 JWT 行为（明确设计）

- 永久封禁 / 临时封禁：`users.status = 'disabled'/'suspended'` + `auth_version+1` + 撤销全部 refresh token → **现有 JWT 立即失效**（`AuthorizationService` 校验 status+auth_version），被禁用户无法登录/刷新。
- 禁言（mute）/ 内容限制（content_restriction）：**不改变用户 status**，`restrictions` 数组生效；后端在每个受限制接口（topic/comment/report/upload）执行处罚检查（`ModerationService::enforce_restrictions(user_id)`，Redis 缓存 60s），不依赖前端隐藏。
- 处罚到期恢复后：用户需重新登录（refresh 已被撤销），安全第一。

---

## 4. API 设计（沿用现有路由规范）

统一：`ApiResponse<T>` 包裹、`{error:{code,message}}` 错误、`Paginated<T>` 分页、`page/page_size`（clamp 1..100）、权限中间件 `AuthorizationLayer`、审计 `AdminAuditContext(ip, ua)`、请求体 20MB 限制。

### 4.1 用户端（认证）

| 方法/路径 | 权限 | 说明 |
| --- | --- | --- |
| POST `/reports` | report.create（已有） | 扩展：reason 枚举校验、优先级自动分级（高风险关键词→high）、防自举报/防重复/限流 |
| GET `/reports/me` | report.create | 我的举报（分页，含状态） |
| GET `/reports/{id}` | 举报人 或 moderation.report.read | 我的举报详情（不暴露审核员内部备注） |
| GET `/moderation/sanctions/me` | — | 我的处罚列表（仅用户可见说明，不含 internal_note） |
| GET `/moderation/sanctions/{id}` | 被处罚人 | 处罚详情 |
| POST `/moderation/sanctions/{id}/appeals` | 被处罚人 | 提交申诉（限次数） |
| GET `/moderation/appeals/me` | — | 我的申诉 |
| GET `/moderation/appeals/{id}` | 申诉人 | 申诉详情 |

### 4.2 管理端（`/admin/moderation/*`，中间件 `admin.access` + 粒度权限）

| 方法/路径 | 权限 | 说明 |
| --- | --- | --- |
| GET `/admin/moderation/reports` | moderation.report.read | 举报队列（status/target_type/reason/priority/assignee/时间筛选 + 分页 + q） |
| GET `/admin/moderation/reports/{id}` | moderation.report.read | 举报详情（快照/相关举报/自动命中/历史） |
| POST `/admin/moderation/reports/{id}/assign` | moderation.report.assign | 领取（并发守卫） |
| POST `/admin/moderation/reports/{id}/release` | moderation.report.assign | 释放 |
| POST `/admin/moderation/reports/{id}/transfer` | moderation.report.assign | 转交 |
| POST `/admin/moderation/reports/{id}/resolve` | moderation.report.review | 接受举报（可联动内容动作+处罚） |
| POST `/admin/moderation/reports/{id}/reject` | moderation.report.review | 拒绝（必须填原因） |
| POST `/admin/moderation/reports/{id}/duplicate` | moderation.report.review | 标记重复 |
| POST `/admin/moderation/reports/batch` | 逐项校验 | 批量（逐项结果返回） |
| GET `/admin/moderation/cases` | moderation.report.read | **统一审核队列**（status/priority/source/type/assignee/q） |
| GET `/admin/moderation/cases/{id}` | moderation.report.read | 案例详情（内容快照+相关举报+作者违规史+命中+历史+备注） |
| POST `/admin/moderation/cases/{id}/assign\|release\|transfer\|note\|close` | 相应权限 | 任务操作 |
| POST `/admin/moderation/topics/{id}/actions` | 见 §2.5 | hide/restore/delete/lock/unlock/pin/unpin/move_category/mark_sensitive/restrict_interactions |
| POST `/admin/moderation/comments/{id}/actions` | 见 §2.5 | hide/restore/delete/collapse/uncollapse/mark_sensitive/restrict_replies |
| GET `/admin/moderation/users/{id}/sanctions` | moderation.report.read | 用户违规记录 |
| POST `/admin/moderation/users/{id}/sanctions` | 见 §2.5 | 执行处罚（warn/mute/suspend/ban + restrictions + 期限） |
| POST `/admin/moderation/sanctions/{id}/revoke` | moderation.sanction.revoke | 解除处罚（永久封禁需 super + 二次确认字段） |
| GET `/admin/moderation/appeals` | moderation.appeal.read | 申诉队列 |
| POST `/admin/moderation/appeals/{id}/review` | moderation.appeal.review | 处理申诉（通过→自动恢复/解禁；拒绝必须填原因） |
| GET/POST/PATCH/DELETE `/admin/moderation/rules[/{id}]` | moderation.rule.manage | 自动审核规则（含敏感词/域名 CRUD，修改写 admin_logs） |
| GET `/admin/moderation/audit-logs` | moderation.audit.read | 治理操作日志（复用 admin_logs，action 前缀 `moderation.*`） |
| GET `/admin/moderation/metrics` | moderation.metrics.read | 治理统计（DB 聚合） |
| GET `/metrics`（Prometheus 文本） | 内部（无鉴权或网络层限制） | 零依赖手工文本格式 |

### 4.3 自动审核钩子（不改公开路由）

- `POST /topics`（TopicService::create）与 `POST /topics/{topic_id}/comments`（CommentService::create_root/reply）内调用 `ModerationService::screen_content()`：
  - 命中 `reject` → 返回 Validation 错误；
  - 命中 `hide` → 内容入库但 status=hidden + 建 case + 通知审核员；
  - 命中 `queue/flag` → 仅记录命中 + 建 case（内容保持 published）；
  - 命中 `rate_limit` → 返回 RateLimited；
  - 命中 `collapse` → comment is_collapsed=true。
- 规则引擎运行于 Redis 缓存规则集（60s TTL），关键词匹配做归一化（去空格/全角转半角/小写）防绕过；频率类用 Redis incr 窗口计数；重复内容用 Redis 内容 hash 集合。
- **自动审核结果可解释**：命中行包含 rule_id + 风险分 + 片段；case 详情展示全部命中。

---

## 5. 前端页面结构

### 5.1 用户端

| 路由 | 组件 | 说明 |
| --- | --- | --- |
| `/topics/[slug]`（扩展） | `topic-view.tsx` | 举报按钮（topic）；锁定提示横幅 |
| `comment-section.tsx`（扩展） | — | 每条评论"举报"菜单；被折叠评论折叠展示 |
| `/profile/reports` | `reports-view.tsx` | 我的举报列表（分页、状态筛选） |
| `/profile/reports/[id]` | `report-detail.tsx` | 我的举报详情 |
| `/profile/sanctions` | `sanctions-view.tsx` | 我的处罚记录（active/expired/revoked） |
| `/profile/sanctions/[id]` | `sanction-detail.tsx` | 处罚详情 + 申诉入口 |
| `/profile/appeals` | `appeals-view.tsx` | 我的申诉 |
| `/profile/appeals/[id]` | `appeal-detail.tsx` | 申诉详情 |
| 举报表单 | `report-form.tsx`（内联 Dialog 风格，无新组件依赖） | 原因下拉（10 类）+ 补充说明；防重复提交 |

交互约束：加载/空/错误/权限状态齐全；表单前后端双重校验（zod + 服务端）；被限制功能显示明确原因；不展示内部备注与举报人身份。

### 5.2 管理端（扩展 `/admin`）

| 路由 | 组件 | 说明 |
| --- | --- | --- |
| `/admin/moderation` | `moderation-overview.tsx` | 治理概览（待处理/积压/近期趋势） |
| `/admin/moderation/reports` | `moderation-reports.tsx` | 举报队列（筛选+分页+批量） |
| `/admin/moderation/reports/[id]` | `moderation-report-detail.tsx` | 举报详情（快照/历史/相关举报/操作） |
| `/admin/moderation/cases` | `moderation-cases.tsx` | 统一审核队列 |
| `/admin/moderation/cases/[id]` | `moderation-case-detail.tsx` | 案例详情 + 内容动作 + 处罚 + 备注 + 批量 |
| `/admin/moderation/sanctions` | `moderation-sanctions.tsx` | 处罚管理（含用户历史） |
| `/admin/moderation/appeals` | `moderation-appeals.tsx` | 申诉队列 |
| `/admin/moderation/appeals/[id]` | `moderation-appeal-detail.tsx` | 申诉处理 |
| `/admin/moderation/rules` | `moderation-rules.tsx` | 自动审核规则 + 敏感词 + 域名管理 |
| `/admin/moderation/logs` | `moderation-logs.tsx` | 治理操作日志（复用 admin_logs） |
| `/admin/moderation/metrics` | `moderation-metrics.tsx` | 治理统计图表（纯前端渲染聚合数据） |

- `admin-shell.tsx` 增加"治理"导航组；`RequireAdmin` 扩展为 `RequireStaff`（moderator+），非治理页面仍按角色隐藏入口。
- 高风险操作（永久封禁/批量删除/批量处罚/解除永久封禁）二次确认（现有 `window.confirm` 模式或内联确认面板）。

---

## 6. 搜索 / SEO / 监控集成

- **搜索**：公开查询已过滤 `status='published'`；确认 comment 查询排除 `hidden`；锁定的 topic 正常可搜。管理后台列表可搜全部状态（现有实现）。
- **SEO**：sitemap/rss/JSON-LD 均经公开 API → 已自动排除 hidden/deleted。无需改前端 SEO 代码。
- **监控**：`/metrics` 零依赖 Prometheus 文本端点，指标：
  - `moderation_reports_total{status}`、`moderation_reports_pending`
  - `moderation_review_duration_seconds`（summary，处理时长）
  - `moderation_actions_total{action,target_type}`
  - `moderation_auto_rules_triggered_total{rule_type,action}`
  - `moderation_sanctions_active{type}`、`moderation_appeals_total{status}`
  - `moderation_websocket_events_total{event_type}`
  - 标签仅低基数（状态/原因类/动作类型），**不含 user_id/topic_id**。
- **统计接口** `/admin/moderation/metrics`：DB 聚合（举报/处理/平均时长/原因分布/命中/处罚/申诉/通过率/审核员工作量/积压量），独立小查询 + 索引支撑，避免影响主业务。

---

## 7. 计划修改文件清单（步骤 ②～⑰）

### 后端 `apps/api`
- `migrations/20260812000000_moderation.up.sql` / `.down.sql`（新）
- `src/models/moderation.rs`（新，治理领域类型 + 状态枚举）
- `src/models/rbac.rs`（新增权限常量 + `ROLE_SENIOR_MODERATOR`）
- `src/models/mod.rs`（导出）
- `src/repositories/moderation.rs`（新）
- `src/repositories/mod.rs`（导出）
- `src/services/moderation.rs`（新，核心服务）
- `src/services/moderation_metrics.rs`（新，统计 + Prometheus 文本）
- `src/services/mod.rs`（导出）
- `src/routes/moderation.rs`（新，用户端 + 管理端路由）
- `src/routes/mod.rs`（挂载）
- `src/routes/admin.rs`（`reports` 相关签名微调，保持兼容）
- `src/services/topic.rs` / `comment.rs`（自动审核钩子 + 锁定/折叠字段校验）
- `src/services/notification.rs` / `src/events/mod.rs`（治理通知类型）
- `src/models/notification.rs`（NotificationType 扩展）
- `src/state.rs`（ModerationService 组装 + 后台任务 spawn）
- `src/error.rs`（新增治理错误映射，复用统一格式）
- `src/lib.rs`（导出新模块）
- `tests/moderation.rs`（新，集成测试）
- `Cargo.toml`（如需新依赖——优先零新增；metrics 用 std）

### 前端 `apps/web`
- `packages/types/src/api.ts`（治理类型扩展）
- `src/lib/api/moderation.ts`（新，用户端 API）
- `src/lib/api/admin-moderation.ts`（新，管理端 API）
- `src/components/moderation/*`（用户端组件）
- `src/components/admin/moderation-*.tsx`（管理端组件）
- `src/app/profile/reports|sanctions|appeals/*`（新路由）
- `src/app/admin/moderation/*`（新路由）
- `src/components/admin/admin-shell.tsx`（治理导航）
- `src/components/auth/route-guards.tsx`（RequireStaff）
- `src/components/forum/topic-view.tsx` / `comment-section.tsx`（举报入口 + 锁定提示 + 折叠展示）
- `src/lib/realtime/types.ts` / `realtime-provider.tsx`（治理事件类型）
- `src/components/forum/notifications-view.tsx`（治理通知类型映射）

### 文档 `docs/moderation/`
- `architecture.md`（本文件）、`database.md`、`permissions.md`、`report-workflow.md`、`moderation-workflow.md`、`sanctions.md`、`appeals.md`、`auto-moderation.md`、`api.md`、`operations.md`、`security.md`、`testing.md`

---

## 8. 风险与回滚

- 迁移**向前兼容**：全部为 `ADD COLUMN`/`ALTER CHECK`/`CREATE TABLE`，无字段删除；`reports` 扩展不破坏现有 admin UI（status 枚举向后兼容）。
- `ALTER TABLE ... DROP CONSTRAINT + ADD CONSTRAINT`（topics/comments/notifications 状态 CHECK）短暂持有 ACCESS EXCLUSIVE 锁；表规模小，迁移执行快；文档标注低谷窗口执行。
- 回滚：提供 `.down.sql`（删除新表/列/约束、恢复原 CHECK）。
- 通知唯一索引：`dedup_key` 为空的行不受约束影响，存量数据零影响。

---

**待确认**：以上设计是否符合你对现有架构的认知？确认后我将按 ②数据库模型与 Migration → ③RBAC → …… → ⑰文档 的顺序逐步实现，每一步先读代码、说明设计、列修改清单、实现、格式化/类型检查/测试、汇报后停下等待确认。
