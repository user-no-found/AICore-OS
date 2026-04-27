# AICore OS 底层规范

## 职责

底层提供跨内核、组件和应用共享的基础 primitive。底层只表达稳定、通用、与业务无关的能力，包括 ID、路径、错误、取消、队列、租约、时间和 redaction。

## 编译整体

底层编译整体是 `crates/foundation/aicore-foundation`。其他层通过该 crate 使用底层 primitive。

## ID primitive

ID primitive 使用强类型 newtype 表达系统身份、调用、事件、任务和 worker 等标识。默认 ID 使用安全 token 规则：非空，并且只允许 ASCII 字母、数字、`.`、`-`、`_`。

## 路径 primitive

路径 primitive 负责生成 AICore OS 的本地目录布局。默认状态根位于用户目录下的 `.aicore`，并派生 instance、config、secrets、components、run、logs 等根目录。

## 错误 primitive

底层错误使用稳定的机器可识别 variant。错误应保留可读 display 文本，同时让上层能够区分重复、缺失、权限拒绝、队列满、取消、超时、版本不匹配、不可用和冲突等场景。

## 取消 primitive

取消 primitive 使用可克隆 token 表达共享取消信号。任意 clone 调用取消后，其他 clone 都必须能观察到取消状态。

## 队列 primitive

队列 primitive 提供有界 FIFO 队列。达到容量后，push 返回队列满错误；pop 返回最早进入的元素。

## 租约 primitive

租约 primitive 表达资源占用状态。租约包含租约 ID、持有者、获得时间、可选过期时间和状态。租约状态包括 Active、Released、Expired、Revoked。

## 时间与超时 primitive

时间 primitive 提供稳定时间戳和 clock trait。系统 clock 使用 Unix millis 表示时间，便于跨层记录和比较。

## 安全与 redaction primitive

底层 primitive 不承载 raw secret。需要进入错误、事件、surface 的内容必须使用已确认可展示的安全摘要或 redacted 文本。

## 底层不得包含的内容

底层不得包含 provider 协议、工具执行、记忆业务、应用路由、会话控制、TUI/Web 展示逻辑或外部网络请求。
