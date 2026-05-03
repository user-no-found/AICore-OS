import { createApp, h } from 'vue'
import './styles.css'

const statusCards = [
  ['运行时', '未接入', '等待 Unified I/O snapshot'],
  ['局域网', '已预留', 'Rust server 默认监听 0.0.0.0:8731'],
  ['工具', '禁用', '不会从 Web 直接执行工具'],
  ['记忆', '只读占位', '不会从 Web 直接写入记忆'],
]

const initialTimeline = [
  {
    role: '系统',
    title: 'Web 控制台已启动',
    body: '当前页面是 Vue3 产品壳，Rust 后端只提供静态资源和状态接口。',
    tone: 'system',
  },
  {
    role: '助手',
    title: '统一消息流预留',
    body: '未来 TUI 与 Web 会订阅同一个 instance 的 Unified I/O stream。',
    tone: 'assistant',
  },
  {
    role: '代码',
    title: 'Rust 后端边界',
    body: 'Browser -> aicore-web Rust server -> Unified I/O -> Kernel Runtime',
    tone: 'code',
  },
]

const button = (label, props = {}) => h('button', props, label)

const App = {
  data() {
    return {
      statusCards,
      timeline: [...initialTimeline],
      composer: '',
      toast: '当前是 Web 预留界面，不启动智能体运行时。',
    }
  },
  methods: {
    submitPreview() {
      if (!this.composer.trim()) return
      this.timeline.push({
        role: '用户',
        title: '本地预览输入',
        body: this.composer.trim(),
        tone: 'user',
      })
      this.composer = ''
      this.toast = '输入只加入本地预览，尚未发送到 Agent Runtime。'
    },
    updateComposer(event) {
      this.composer = event.target.value
    },
  },
  render() {
    return h('main', { class: 'shell' }, [
      h('aside', { class: 'rail' }, [
        h('div', { class: 'brand' }, [
          h('span', { class: 'mark' }, 'A'),
          h('div', [h('strong', 'AICore OS'), h('small', 'Web Console')]),
        ]),
        h('nav', [
          button('控制台', { class: 'active' }),
          button('实例'),
          button('审批'),
          button('媒体'),
          button('设置'),
        ]),
        h('section', { class: 'lan' }, [
          h('span', '局域网访问'),
          h('strong', '预留开启'),
          h('small', 'http://0.0.0.0:8731'),
        ]),
      ]),

      h('section', { class: 'workspace' }, [
        h('header', { class: 'topbar' }, [
          h('div', [h('p', '当前实例'), h('h1', 'global-main / Web 预览')]),
          h('div', { class: 'status-pill' }, 'Vue3 页面 · Rust 后端 · 未接 Agent'),
        ]),
        h('section', { class: 'hero' }, [
          h('div', [
            h('span', { class: 'eyebrow' }, 'AICore Web Surface'),
            h('h2', '为 FNOS 与浏览器预留的现代 AI 控制台'),
            h(
              'p',
              '页面由 Vue3 构建；与内核、会话、记忆、工具和 Unified I/O 的交互统一由 Rust 后端接入。',
            ),
          ]),
          h('div', { class: 'orb' }),
        ]),
        h(
          'section',
          { class: 'cards' },
          this.statusCards.map((card) =>
            h('article', { key: card[0] }, [
              h('span', card[0]),
              h('strong', card[1]),
              h('p', card[2]),
            ]),
          ),
        ),
        h(
          'section',
          { class: 'timeline' },
          this.timeline.map((item) =>
            h('article', { key: `${item.title}-${item.body}`, class: ['message', item.tone] }, [
              h('header', [h('span', item.role), button('复制')]),
              h('h3', item.title),
              h('p', item.body),
            ]),
          ),
        ),
      ]),

      h('aside', { class: 'inspector' }, [
        h('section', [h('h2', '运行检查器'), h('p', 'Foundation / Kernel runtime 尚未连接。')]),
        h('section', [
          h('h2', '审批'),
          h('p', '暂无待审批请求。后续工具、文件、记忆写入会在这里确认。'),
        ]),
        h('section', [
          h('h2', '媒体与文件'),
          h('p', '图片、视频、代码片段和 artifact 预览区域已预留。'),
        ]),
        h('section', { class: 'toast' }, this.toast),
      ]),

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
          h('div', [
            button('附件', { type: 'button' }),
            button('停止', { type: 'button' }),
            button('发送预览', { type: 'submit' }),
          ]),
        ],
      ),
    ])
  },
}

createApp(App).mount('#app')
