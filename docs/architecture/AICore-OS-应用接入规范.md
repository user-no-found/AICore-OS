# AICore OS 应用接入规范

## 应用层定义

应用层是使用内核公开合同提供交互、能力或组件服务的上层单元。CLI、TUI、Web、Provider、Toolset、Gateway 和 Service 都属于应用层或组件层。

## 应用整体包规则

应用应作为独立可编译单元存在。应用可以依赖 `aicore-kernel` 与 `aicore-foundation`，并通过公开合同访问内核能力。

## 应用注册

应用注册使用 AppManifest。注册信息包括 app_id、runtime kind、合同版本、能力描述和权限边界。

## 应用握手

应用握手使用 AppHandshake。握手期间，应用向内核声明自身支持的合同和能力，内核据此进行兼容性与路由判断。

## 能力声明

能力声明使用 CapabilityDescriptor。每项能力必须声明 capability_id 和 operations，并可附带 schema、credential 和 sandbox 要求。

## 调用 envelope

内核调用应用能力时使用 KernelInvocationEnvelope。应用不得绕过 envelope 读取内部内核状态。

应用作为 component process 接入内核时，应通过 installed manifest 声明 invocation mode、transport、entrypoint、args 和 capability。`stdio_jsonl` component process 的 stdout 只承载协议 result，不承载 human panel。业务只读 handler 应返回结构化 public fields，并由 CLI、TUI 或其他 surface 从 result envelope 派生人类摘要。

`config.validate`、`auth.list`、`model.show`、`service.list`、`runtime.smoke`、`instance.list`、`cli.status`、`provider.smoke`、`agent.smoke` 与 `agent.session_smoke` 类只读或 smoke 业务能力可以由 application binary 暴露内部 stdio handler，并通过 Kernel runtime binary 调用。该内部 handler 不属于用户产品命令，不应出现在普通帮助 surface 中。handler 输出必须是单行 JSONL result，且不得输出 human panel、ANSI 或 raw credential material。

Agent smoke 类 handler 只表达 agent loop/session 的只读诊断摘要，不代表 real provider、streaming、tool calling 或 Memory Agent 已接入。其 public result 可以包含 conversation id、outcome、event count、queue length、provider invoked 状态、assistant output presence、turn count、stop reason、kernel invocation path、`real_provider=false`、`tool_calling=false` 与 `streaming=false` 等安全字段。public surface、JSON result、diagnostic 和 ledger 不得输出 full prompt、raw memory pack、raw provider request、raw provider payload、raw assistant content、secret、`secret_ref`、credential lease、API key、token 或 cookie。

## 事件 envelope

应用向内核报告事件时使用 KernelEventEnvelope。事件 visibility 决定事件面向内部、用户还是审计。

## 错误返回

应用错误返回应映射为 KernelError。用户可见消息使用中文，机器字段使用稳定英文 enum。错误不得泄漏 raw secret、secret_ref 或内部 request。

## 热插拔与兼容

应用可在 turn boundary 或安全边界进行启停、升级和移除。内核通过合同版本、能力描述和路由决策判断兼容性。

## 正式文档维护规则

正式说明文档只记录稳定规格。过程记录、阶段状态、验证细节和协作日志属于外部规划工作区。
