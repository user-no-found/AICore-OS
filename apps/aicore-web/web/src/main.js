import { createApp } from 'vue'
import './styles.css'

const statusCards = [
  ['运行时', '未接入', '等待 Unified I/O snapshot'],
  ['局域网', '已预留', 'Rust server 默认监听 0.0.0.0:8731'],
  ['工具', '禁用', '不会从 Web 直接执行工具'],
  ['记忆', '只读占位', '不会从 Web 直接写入记忆'],
]

const timeline = [
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

const App = {
  data() {
    return {
      statusCards,
      timeline,
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
  },
  template: `
    <main class="shell">
      <aside class="rail">
        <div class="brand">
          <span class="mark">A</span>
          <div>
            <strong>AICore OS</strong>
            <small>Web Console</small>
          </div>
        </div>
        <nav>
          <button class="active">控制台</button>
          <button>实例</button>
          <button>审批</button>
          <button>媒体</button>
          <button>设置</button>
        </nav>
        <section class="lan">
          <span>局域网访问</span>
          <strong>预留开启</strong>
          <small>http://0.0.0.0:8731</small>
        </section>
      </aside>

      <section class="workspace">
        <header class="topbar">
          <div>
            <p>当前实例</p>
            <h1>global-main / Web 预览</h1>
          </div>
          <div class="status-pill">Vue3 页面 · Rust 后端 · 未接 Agent</div>
        </header>

        <section class="hero">
          <div>
            <span class="eyebrow">AICore Web Surface</span>
            <h2>为 FNOS 与浏览器预留的现代 AI 控制台</h2>
            <p>页面由 Vue3 构建；与内核、会话、记忆、工具和 Unified I/O 的交互统一由 Rust 后端接入。</p>
          </div>
          <div class="orb"></div>
        </section>

        <section class="cards">
          <article v-for="card in statusCards" :key="card[0]">
            <span>{{ card[0] }}</span>
            <strong>{{ card[1] }}</strong>
            <p>{{ card[2] }}</p>
          </article>
        </section>

        <section class="timeline">
          <article v-for="item in timeline" :key="item.title + item.body" :class="['message', item.tone]">
            <header>
              <span>{{ item.role }}</span>
              <button>复制</button>
            </header>
            <h3>{{ item.title }}</h3>
            <p>{{ item.body }}</p>
          </article>
        </section>
      </section>

      <aside class="inspector">
        <section>
          <h2>运行检查器</h2>
          <p>Foundation / Kernel runtime 尚未连接。</p>
        </section>
        <section>
          <h2>审批</h2>
          <p>暂无待审批请求。后续工具、文件、记忆写入会在这里确认。</p>
        </section>
        <section>
          <h2>媒体与文件</h2>
          <p>图片、视频、代码片段和 artifact 预览区域已预留。</p>
        </section>
        <section class="toast">{{ toast }}</section>
      </aside>

      <form class="composer" @submit.prevent="submitPreview">
        <textarea v-model="composer" placeholder="输入消息；当前只进入本地 Web 预览。"></textarea>
        <div>
          <button type="button">附件</button>
          <button type="button">停止</button>
          <button type="submit">发送预览</button>
        </div>
      </form>
    </main>
  `,
}

createApp(App).mount('#app')
