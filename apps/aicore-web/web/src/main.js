import { createApp, h } from 'vue'
import './styles.css'

const navItems = [
  ['总览', 'grid'],
  ['实例', 'instance'],
  ['会话', 'stream'],
  ['记忆', 'memory'],
  ['工具', 'tool'],
  ['Skills/MCP', 'skill'],
  ['团队', 'team'],
  ['审批', 'audit'],
  ['事件账本', 'ledger'],
  ['设置', 'settings'],
]

const runtimeChips = [
  ['Foundation', '运行中', 'ok'],
  ['Kernel', '运行中', 'ok'],
  ['I/O Gateway', '预留中', 'warn'],
  ['Event Ledger', '运行中', 'ok'],
  ['Memory App', '待接入', 'idle'],
  ['Tool Registry', '可发现', 'ok'],
]

const streamEvents = [
  ['10:42:11', 'Web', '用户输入 (Web)', '请分析 /data/sales.csv，生成趋势报告并找出异常点。', 'user', '用户'],
  ['10:42:12', 'Kernel', 'Kernel 事件', '已观测执行步骤 (4)，调度工具、准备读取文件。', 'kernel', '运行中'],
  ['10:42:14', 'Tool', '工具结果：file.read', '已读取 12.4 MB，解析 1,204 行，耗时 1.23s。', 'tool', '成功'],
  ['10:42:16', 'Team', '团队笔记：data-analyst', '检测到 3 个异常点，建议使用 IQR 方法复核。', 'team', '团队'],
  ['10:42:18', 'System', '系统日志', '已写入事件账本，回合 12，耗时 3.67s。', 'system', '记录'],
]

const tools = [
  ['file', '文件读写', 'workspace-write', '需要审批', 'available'],
  ['shell', '命令执行', 'workspace-write', '需要审批', 'available'],
  ['search', '网络搜索', 'network', '需要审批', 'available'],
  ['browser', '浏览器自动化', 'network', '需要审批', 'available'],
  ['mcp', 'MCP 客户端', 'workspace-write', '需要审批', 'available'],
  ['skills', '技能包', 'workspace', '不需审批', 'available'],
  ['memory.search', '记忆检索', 'read-only', '不需审批', 'available'],
]

const memories = [
  ['sales_data_pattern_v1', '事实', 'Sales data shows weekly seasonality with peak on weekends and anomalies on promotions.', '相似度 0.92'],
  ['用户注释（预览）', '注释', '销售数据呈现周末高峰，促销期有异常点。使用 IQR 方法进行异常检测。', 'UI-only'],
]

const approvals = [
  ['shell.exec', 'python analyze.py --full', 'workspace-write', '需要审批'],
  ['file.write', '/reports/summary.md', 'workspace-write', '需要审批'],
]

const ledger = [
  ['10:42:18', 'Turn 12 完成', 'kernel'],
  ['10:42:16', '团队笔记已记录', 'team-bus'],
  ['10:42:14', '工具 file.read 成功', 'tool'],
  ['10:42:12', 'Kernel 开始执行', 'kernel'],
  ['10:41:58', '用户输入 (Turn 12)', 'web'],
]

const agents = [
  ['data-analyst', '数据分析师', '分析、可视化、统计', '活跃'],
  ['report-writer', '报告撰写员', '摘要、洞察、排版', '活跃'],
]

const hText = (text) => text

const button = (label, props = {}) => h('button', props, label)

const chip = (label, value, tone = 'idle') =>
  h('div', { class: ['chip', tone] }, [h('span', label), h('strong', value)])

const Panel = (title, children, props = {}) =>
  h('section', { class: ['panel', props.class], key: props.key }, [
    h('header', { class: 'panel-head' }, [
      h('h2', title),
      props.action ? h('span', { class: 'panel-action' }, props.action) : null,
    ]),
    ...children,
  ])

const App = {
  data() {
    return {
      composer: '',
      toast: '当前是 Web 产品壳：只做本地预览，不启动 Agent Runtime。',
      streamEvents: [...streamEvents],
    }
  },
  methods: {
    submitPreview() {
      const value = this.composer.trim()
      if (!value) return
      this.streamEvents.push(['10:42:30', 'Web', '本地预览输入', value, 'user', '排队'])
      this.composer = ''
      this.toast = '输入已加入本地预览队列，尚未发送到 Unified I/O。'
    },
    updateComposer(event) {
      this.composer = event.target.value
    },
  },
  render() {
    return h('main', { class: 'aicore-shell' }, [
      h('aside', { class: 'side-rail' }, [
        h('div', { class: 'brand-block' }, [
          h('span', { class: 'brand-mark' }, 'AC'),
          h('div', [h('strong', 'AICore OS'), h('small', 'Web Console')]),
        ]),
        h(
          'nav',
          { class: 'nav-stack' },
          navItems.map(([label, icon], index) =>
            button(label, { class: index === 0 ? 'active' : '', 'data-icon': icon }),
          ),
        ),
        h('section', { class: 'instance-card' }, [
          h('span', '当前实例'),
          h('strong', 'inst_7f3a9c2e'),
          h('p', 'workspace / acme / research'),
          h('div', [h('i'), hText('运行中')]),
        ]),
      ]),

      h('section', { class: 'main-grid' }, [
        h('header', { class: 'topbar' }, [
          h('div', { class: 'crumbs' }, [
            h('span', '工作区 / acme / research / default'),
            h('strong', '实例 ID：inst_7f3a9c2e'),
          ]),
          h(
            'div',
            { class: 'runtime-strip' },
            runtimeChips.map(([label, value, tone]) => chip(label, value, tone)),
          ),
          h('div', { class: 'network-card' }, [
            h('span', 'fnOS 原生包'),
            h('strong', 'LAN 访问'),
            h('small', 'http://192.168.1.108:8731'),
          ]),
        ]),

        Panel('Unified I/O Stream', [
          h('div', { class: 'stream-toolbar' }, [
            h('div', [h('span', 'Web'), h('span', 'TUI'), h('b', '同步中')]),
            h('div', [button('实时'), button('折叠'), button('全屏')]),
          ]),
          h(
            'div',
            { class: 'stream-list' },
            this.streamEvents.map(([time, source, title, body, tone, state]) =>
              h('article', { class: ['stream-row', tone], key: `${time}-${title}-${body}` }, [
                h('time', time),
                h('span', { class: 'source' }, source),
                h('div', [h('strong', title), h('p', body)]),
                h('em', state),
              ]),
            ),
          ),
          h('div', { class: 'pending-strip' }, [
            h('span', '待输入队列 (1)'),
            h('p', '请基于结果生成可视化图表，并标注异常点。'),
            button('取消'),
          ]),
          button('停止当前执行', { class: 'danger-button' }),
        ], { class: 'stream-panel', action: '当前回合已锁定 (Turn 12)' }),

        Panel('Runtime Topology', [
          h('div', { class: 'topology' }, [
            h('div', { class: 'edge-clients' }, [h('span', 'Web'), h('span', 'TUI'), h('span', 'CLI')]),
            h('div', { class: 'runtime-nodes' }, [
              h('div', { class: 'node cyan' }, [h('b'), h('span', 'Foundation Runtime')]),
              h('div', { class: 'node blue' }, [h('b'), h('span', 'Kernel Runtime')]),
              h('div', { class: 'node green' }, [h('b'), h('span', 'Instance Runtime'), h('small', 'inst_7f3a9c2e')]),
              h('div', { class: 'component-row' }, [
                h('span', 'Memory App'),
                h('span', 'Tool Registry'),
                h('span', 'Event Ledger'),
                h('span', 'Skills/MCP'),
              ]),
            ]),
            h('div', { class: 'edge-services' }, [
              h('span', 'Tools'),
              h('span', 'Providers'),
              h('span', 'Memory'),
              h('span', 'Team Agents'),
            ]),
          ]),
        ], { class: 'topology-panel', action: '拓扑视图' }),

        Panel('Approval & Audit', [
          h(
            'div',
            { class: 'approval-list' },
            approvals.map(([name, body, scope, state]) =>
              h('article', { key: name }, [
                h('header', [h('strong', name), h('span', state)]),
                h('p', body),
                h('small', scope),
                h('footer', [button('拒绝'), button('批准')]),
              ]),
            ),
          ),
          h('div', { class: 'audit-state' }, [
            h('h3', '审批状态（First-Writer-Wins）'),
            h('p', 'shell.exec 已批准，net.fetch 由 team-lead 批准。'),
          ]),
          h(
            'div',
            { class: 'ledger-list' },
            ledger.map(([time, title, source]) => h('p', { key: `${time}-${title}` }, [h('time', time), h('span', title), h('small', source)])),
          ),
        ], { class: 'audit-panel', action: '全部' }),

        Panel('Memory & Proposals', [
          h('div', { class: 'segmented' }, [button('记忆检索', { class: 'active' }), button('提案审阅 (3)')]),
          h('label', { class: 'search-line' }, [h('span', '⌕'), h('input', { value: '搜索实例记忆...', readonly: true })]),
          h(
            'div',
            { class: 'memory-list' },
            memories.map(([title, tag, body, meta]) =>
              h('article', { key: title }, [h('header', [h('strong', title), h('span', tag)]), h('p', body), h('small', meta)]),
            ),
          ),
        ], { class: 'memory-panel' }),

        Panel('Tool / Skill Hot-Plug', [
          h(
            'div',
            { class: 'tool-grid' },
            tools.map(([id, title, scope, approval, state]) =>
              h('article', { key: id }, [
                h('header', [h('strong', id), h('span', state === 'available' ? '可用' : '失效')]),
                h('p', title),
                h('small', `沙箱：${scope}`),
                h('small', `审批：${approval}`),
                h('em', '3 回合内有效'),
              ]),
            ),
          ),
        ], { class: 'tool-panel', action: '全部工具' }),

        Panel('Team Runtime', [
          h(
            'div',
            { class: 'agent-list' },
            agents.map(([name, role, ability, state]) =>
              h('article', { key: name }, [
                h('div', { class: 'avatar' }, name.slice(0, 2).toUpperCase()),
                h('div', [h('strong', name), h('p', role), h('small', ability)]),
                h('span', state),
              ]),
            ),
          ),
          h('div', { class: 'team-bus' }, [
            h('h3', '代理总线（实时消息）'),
            h('p', 'data-analyst 检测到 3 个异常点，建议复核。'),
            h('p', 'report-writer 已准备生成报告结构。'),
          ]),
        ], { class: 'team-panel', action: '全部代理' }),

        h(
          'form',
          {
            class: 'composer',
            onSubmit: (event) => {
              event.preventDefault()
              this.submitPreview()
            },
          },
          [
            h('textarea', {
              value: this.composer,
              placeholder: '输入消息；当前只进入本地 Web 预览。',
              onInput: this.updateComposer,
            }),
            h('div', [button('附件', { type: 'button' }), button('停止', { type: 'button' }), button('发送预览', { type: 'submit' })]),
            h('p', this.toast),
          ],
        ),
      ]),

      h('footer', { class: 'system-footer' }, [
        h('span', '系统状态：健康'),
        h('span', 'CPU 18%'),
        h('span', '内存 42%'),
        h('span', '网络 ↑34 KB/s ↓28 KB/s'),
        h('span', 'Rust 1.95.0'),
        h('span', 'Vue 3.5'),
        h('span', '时区 Asia/Shanghai'),
      ]),
    ])
  },
}

createApp(App).mount('#app')
