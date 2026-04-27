# AICore OS 内核协议规范

## 职责

内核协议定义应用、能力、调用、事件、路由、错误、版本、权限、审计和 trace 的稳定合同。应用通过内核协议接入系统，不直接依赖内核内部源码边界。

## 编译整体

内核编译整体是 `crates/kernel/aicore-kernel`。应用层通过 `aicore-kernel` 使用内核公开类型和运行时合同。

## AppManifest

`AppManifest` 描述应用 ID、运行时类型、显示名称、合同版本、能力声明和权限边界。应用 ID 必须稳定，运行时类型用于区分 CLI、TUI、Web、Provider、Toolset、Gateway 和 Service 等形态。

## AppHandshake

`AppHandshake` 是应用启动或注册时交给内核的握手信息。它声明应用支持的合同版本和能力描述。

## CapabilityDescriptor

`CapabilityDescriptor` 描述能力 ID、操作列表、schema 引用、凭证要求和 sandbox 要求。能力操作使用英文机器名。

## KernelInvocationEnvelope

`KernelInvocationEnvelope` 是内核调用应用能力的标准 envelope。它必须携带 instance_id、capability、operation、payload、policy、trace_context 和 audit_context。

## KernelEventEnvelope

`KernelEventEnvelope` 是内核事件标准 envelope。它必须携带 event_id、event_type、instance_id、app_id、invocation_id、visibility、payload 和 trace_context。

## KernelRouteRequest

`KernelRouteRequest` 表达一次能力路由请求。它携带 instance、capability、operation、可选合同版本要求、trace context 和 audit context。

## KernelRouteDecision

`KernelRouteDecision` 表达路由结果。它携带目标 app、目标合同版本、路由策略、路由原因和 fallback chain。

## KernelError

`KernelError` 包含机器错误码、错误阶段、中文用户消息、安全详情、重试提示和 secret-safe 标记。错误不得携带 raw secret 或内部 provider request。

## ContractVersion

`ContractVersion` 使用 contract_id、major 和 minor 表达合同版本。兼容范围通过合同 ID 与 major 范围判定。

## PermissionBoundary

`PermissionBoundary` 描述应用或组件的权限范围和能力边界。需要审批的能力必须显式标记。

## AuditContext

`AuditContext` 描述调用的发起者和原因。高影响调用必须带有可审计原因。

## TraceContext

`TraceContext` 描述 trace_id 和可选父 span。跨应用、跨组件调用应保持 trace 连续。
