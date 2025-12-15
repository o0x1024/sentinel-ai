export default {
  title: '被动扫描',
  tabs: {
    control: '代理控制',
    history: '历史记录',
    repeater: '重放器',
    proxifier: '代理工具',
    capture: '抓包',
    proxyConfig: '代理配置'
  },
  ariaLabels: {
    passiveScanTabs: '被动扫描标签页'
  },
  tooltips: {
    control: '代理控制',
    history: '历史记录',
    repeater: '重放器',
    proxifier: '代理工具',
    capture: '抓包',
    proxyConfig: '代理配置'
  },
  icons: {
    control: 'fa-sliders-h',
    history: 'fa-history',
    repeater: 'fa-redo',
    proxifier: 'fa-network-wired',
    capture: 'fa-broadcast-tower',
    proxyConfig: 'fa-cog'
  },
  // Proxy Intercept Component
  intercept: {
    title: '拦截',
    status: {
      on: '拦截已开启',
      off: '拦截已关闭'
    },
    buttons: {
      turnOn: '开启',
      turnOff: '关闭'
    },
    stats: {
      proxyStatus: '代理状态',
      running: '运行中',
      stopped: '已停止',
      port: '端口',
      interceptQueue: '拦截队列'
    },
    waiting: '等待被拦截的请求/响应...',
    disabled: '请先启用拦截功能',
    proxyConfig: '请将浏览器代理配置为',
  },
  // Proxy Control Component
  control: {
    title: '代理控制',
    stats: {
      proxyStatus: '代理状态',
      running: '运行中',
      stopped: '已停止',
      notStarted: '未启动',
      mitmStatus: 'MITM 状态',
      enabled: '已启用',
      disabled: '未启用',
      mitmDesc: '中间人拦截',
      qps: 'QPS',
      qpsDesc: '每秒请求数',
      requestStats: '请求统计',
      http: 'HTTP',
      https: 'HTTPS'
    }
  },
  // Proxy History Component
  history: {
    title: '请求历史',
    emptyState: {
      noRequests: '暂无请求记录',
      selectInterface: '请选择网络接口'
    },
    toolbar: {
      filter: '过滤',
      refresh: '刷新',
    },
    filterDialog: {
      title: '过滤设置',
      search: '搜索',
      method: '方法',
      host: '主机',
      url: 'URL',
      statusCode: '状态码',
      contentType: '内容类型',
      contains: '包含',
      startTime: '开始时间',
      endTime: '结束时间',
      reset: '重置',
      apply: '应用',
      checkboxes: {
        showOnlyWithParams: '仅显示带参数的请求',
        hideWithoutResponse: '隐藏无响应项',
        html: 'HTML',
        javascript: 'JavaScript',
        css: 'CSS',
        json: 'JSON',
        xml: 'XML',
        image: '图片',
        other: '其他',
        otherText: '其他文本',
        script: '脚本',
        images: '图片',
        flash: 'Flash',
        otherBinary: '其他二进制',
        port: '端口',
        regex: '正则',
        caseSensitive: '区分大小写',
        negativeSearch: '反向搜索',
        placeholders: {
          port: '例如 8080',
          search: '搜索...',
          showExtensions: 'asp,aspx,jsp,php',
          hideExtensions: 'js,gif,jpg,png,css',
        },
        labels: {
          showOnly: '仅显示：',
          hide: '隐藏：',
        },
      },
    },
    detailsPanel: {
      requestDetails: '请求详情',
      responseDetails: '响应详情',
      tabs: {
        headers: 'Headers',
        raw: 'Raw',
        pretty: 'Pretty',
        hex: 'Hex'
      },
      request: '请求',
      response: '响应',
      decompressed: '已解压',
      close: '关闭',
    },
    table: {
      id: 'ID',
      time: '时间',
      method: '方法',
      host: '主机',
      url: 'URL',
      status: '状态',
      length: '长度',
      mimeType: 'MIME类型',
      actions: '操作',
    },
    contextMenu: {
      sendToRepeater: '发送到重放器',
      sendRequestToAssistant: '发送请求到助手',
      sendResponseToAssistant: '发送响应到助手',
      copyUrl: '复制 URL',
      copyAsCurl: '复制为 cURL'
    }
  },
  // Proxy Repeater Component
  repeater: {
    contextMenu: {
      sendRequest: '发送请求',
      sendToNewTab: '发送到新标签页',
      copyUrl: '复制 URL',
      copyRequest: '复制请求',
      copyAsCurl: '复制为 cURL',
      sendRequestToAssistant: '发送请求到助手'
    }
  },
  // Packet Capture Component
  capture: {
    toolbar: {
      interfaceSelect: '网卡选择',
      selectInterface: '选择网卡',
      start: '开始',
      clear: '清空',
      open: '打开',
      save: '保存',
      export: '导出',
      advancedFilter: '高级过滤',
      filterInput: '过滤器输入',
      statistics: '统计信息',
      advancedFiltering: '高级过滤'
    },
    table: {
      time: '时间',
      source: '源地址',
      destination: '目标地址',
      protocol: '协议',
      length: '长度',
      info: '信息',
      no: '序号'
    },
    statusBar: {
      capturing: '抓包中',
      selected: '选中',
      captured: '已捕获',
      packets: '包'
    },
    contextMenu: {
      mark: '标记',
      unmark: '取消标记',
      ignore: '忽略',
      unignore: '取消忽略',
      filter: '过滤',
      filterByField: '按字段过滤',
      sourceAddress: '源地址',
      destinationAddress: '目标地址',
      protocol: '协议',
      conversation: '会话',
      followStream: '追踪流',
      tcpStream: 'TCP 流',
      udpStream: 'UDP 流',
      httpStream: 'HTTP 流',
      copy: '复制',
      summary: '摘要',
      copySource: '源地址',
      copyDestination: '目标地址',
      filterByValue: '按此值过滤'
    },
    emptyStates: {
      gettingInterfaces: '正在获取网卡列表...',
      noInterfaces: '未检测到可用网卡',
      installNpcap: 'Windows 系统需要安装 Npcap 驱动才能进行网络抓包',
      downloadNpcap: '下载 Npcap',
      selectAndStart: '选择网卡并点击开始抓包',
      waitingForPackets: '等待数据包...'
    },
    hexView: {
      hex: '十六进制',
      ascii: 'ASCII',
      raw: '原始数据'
    }
  },
  // Packet Capture Component (new structure)
  packetCapture: {
    toolbar: {
      selectInterface: '选择接口',
      start: '开始',
      stop: '停止',
      clear: '清空',
      open: '打开文件',
      save: '保存文件',
      export: '导出',
      advancedFilter: '高级筛选',
      advancedFilterBadge: '高级',
      filterPlaceholder: '筛选数据包...',
      filtering: '筛选中'
    },
    table: {
      no: '序号',
      time: '时间',
      source: '源地址',
      destination: '目标地址',
      protocol: '协议',
      length: '长度',
      info: '信息'
    },
    statusBar: {
      capturing: '抓包中',
      selected: '选中',
      captured: '已捕获',
      packets: '包'
    },
    emptyState: {
      loadingInterfaces: '正在获取网卡列表...',
      noInterfaces: '未检测到可用网卡',
      npcapRequired: 'Windows 系统需要安装 Npcap 驱动才能进行网络抓包',
      downloadNpcap: '下载 Npcap',
      selectAndStart: '选择网卡并点击开始抓包',
      waitingForPackets: '等待数据包...'
    },
    contextMenu: {
      mark: '标记',
      unmark: '取消标记',
      ignore: '忽略',
      unignore: '取消忽略',
      filter: '过滤',
      filterByField: '按字段过滤',
      sourceAddress: '源地址',
      destinationAddress: '目标地址',
      protocol: '协议',
      conversation: '会话',
      followStream: '追踪流',
      tcpStream: 'TCP 流',
      udpStream: 'UDP 流',
      httpStream: 'HTTP 流',
      copy: '复制',
      summary: '摘要',
      copySource: '源地址',
      copyDestination: '目标地址',
      filterThisValue: '按此值过滤',
      hex: '十六进制'
    },
    filterDialog: {
      title: '高级过滤',
      protocol: '协议',
      sourceIp: '源 IP',
      destinationIp: '目标 IP',
      sourcePort: '源端口',
      destinationPort: '目标端口',
      containsString: '包含字符串',
      containsHex: '包含十六进制',
      minLength: '最小长度',
      maxLength: '最大长度',
      tcpFlags: 'TCP 标志',
      reset: '重置',
      cancel: '取消',
      apply: '应用'
    },
    streamDialog: {
      ascii: 'ASCII',
      hex: '十六进制',
      raw: '原始数据',
      clientToServer: '客户端 → 服务器',
      serverToClient: '服务器 → 客户端',
      packets: '包',
      close: '关闭'
    },
    extractDialog: {
      title: '文件提取',
      foundFiles: '找到 {count} 个文件',
      analyzing: '正在分析数据包以提取文件...',
      supportedProtocols: '支持：HTTP、FTP、邮件附件、DNS 隧道',
      noFilesFound: '当前数据包选择中未找到文件',
      protocolExamples: '示例：HTTP 下载、FTP 传输、邮件附件',
      filterConditions: '过滤条件',
      clearFilter: '清除过滤',
      filename: '文件名',
      searchFilename: '搜索文件名...',
      fileType: '文件类型',
      allTypes: '所有类型',
      image: '图片',
      video: '视频',
      audio: '音频',
      archive: '压缩包',
      document: '文档',
      executable: '可执行文件',
      other: '其他',
      sourceProtocol: '源协议',
      allSources: '所有源',
      fileSize: '文件大小',
      anySize: '任意大小',
      sizeTiny: '极小 (< 1KB)',
      sizeSmall: '小 (1KB - 10KB)',
      sizeMedium: '中等 (10KB - 1MB)',
      sizeLarge: '大 (1MB - 10MB)',
      sizeHuge: '巨大 (> 10MB)',
      http: 'HTTP',
      ftp: 'FTP',
      email: '邮件',
      dnsTunnel: 'DNS 隧道',
      type: '类型',
      size: '大小',
      source: '来源',
      traffic: '流量',
      actions: '操作',
      downloadFile: '下载文件',
      traceTraffic: '追踪流量',
      locatePackets: '定位数据包',
      close: '关闭',
      saveSelectedFiles: '保存选中的文件',
      selectedFiles: '选中的文件',
      selectedSize: '选中大小'
    }
  },
  // Proxy Configuration Component
  configuration: {
    title: '代理监听器',
    description: '配置代理监听器以接收来自浏览器的 HTTP 请求。需要配置浏览器使用其中一个监听器作为代理服务器。',
    table: {
      select: '选择',
      running: '运行中',
      interface: '接口',
      invisible: '不可见',
      redirect: '重定向',
      certificate: '证书',
      tlsProtocol: 'TLS协议',
      supportHttp2: '支持 HTTP/2'
    }
  },
  proxyConfiguration: {
    proxyListenersTitle: '代理监听器',
    proxyListenersDescription: '配置代理监听器以接收来自浏览器的 HTTP 请求。需要配置浏览器使用其中一个监听器作为代理服务器。',
    running: '运行中',
    interface: '接口',
    invisible: '不可见',
    redirect: '重定向',
    certificate: '证书',
    tlsProtocols: 'TLS协议',
    http2Support: '支持 HTTP/2',
    addListener: '添加监听器',
    editListener: '编辑监听器',
    removeListener: '移除监听器',
    exportCACert: '导出 CA 证书',
    regenerateCACert: '重新生成 CA 证书',
    openCertDir: '打开证书目录',
    certInfo: '证书信息',
    bindAddress: '绑定地址',
    port: '端口',
    certMode: '证书模式',
    perHostCert: '按主机证书',
    wildcardCert: '通配符证书',
    customCert: '自定义证书',
    defaultTLS: '默认',
    tls12: 'TLS 1.2',
    tls13: 'TLS 1.3',
    tls12Plus13: 'TLS 1.2 + 1.3',
    supportHTTP2: '支持 HTTP/2',
    invisibleMode: '隐形模式',
    enableRedirect: '启用重定向',
    cancel: '取消',
    save: '保存',
    close: '关闭',
    interceptionRules: '拦截规则',
    interceptionRulesDesc: '配置请求和响应的拦截规则',
    interceptRequests: '拦截请求',
    masterInterceptionDisabled: '主拦截功能已禁用',
    enable: '启用',
    operator: '操作符',
    matchType: '匹配类型',
    relationship: '关系',
    condition: '条件',
    addRule: '添加规则',
    editRule: '编辑规则',
    removeRule: '删除规则',
    moveUp: '上移',
    moveDown: '下移',
    autoFixNewlines: '自动修复换行符',
    autoUpdateContentLength: '自动更新内容长度',
    interceptResponses: '拦截响应',
    autoUpdateResponseContentLength: '自动更新响应内容长度',
    websocketInterceptionRules: 'WebSocket 拦截规则',
    websocketInterceptionRulesDesc: '配置 WebSocket 拦截规则',
    interceptClientToServer: '拦截客户端 → 服务器',
    interceptServerToClient: '拦截服务器 → 客户端',
    onlyInterceptInScope: '仅拦截范围内的项目',
    responseModificationRules: '响应修改规则',
    responseModificationRulesDesc: '配置响应修改规则',
    unhideHiddenFields: '显示隐藏的表单字段',
    prominentlyHighlightUnhidden: '突出显示取消隐藏的字段',
    enableDisabledFields: '启用禁用的表单字段',
    removeInputFieldLengthLimits: '移除输入字段长度限制',
    removeJavaScriptFormValidation: '移除 JavaScript 表单验证',
    removeAllJavaScript: '移除所有 JavaScript',
    matchReplaceRules: '匹配和替换规则',
    matchReplaceRulesDesc: '配置请求和响应的匹配和替换规则',
    onlyApplyToInScope: '仅应用于范围内的项目',
    enabled: '已启用',
    item: '项目',
    match: '匹配',
    replace: '替换',
    type: '类型',
    comment: '注释',
    add: '添加',
    edit: '编辑',
    remove: '删除',
    pasteURL: '粘贴 URL',
    load: '加载',
    tlsPassThrough: 'TLS 直通',
    tlsPassThroughDesc: '配置 TLS 直通规则',
    hostIPRange: '主机/IP 范围',
    noRules: '无规则',
    autoAddTLSOnFailure: '失败时自动添加 TLS',
    applyToOutOfScope: '应用于范围外的项目',
    proxyHistoryLogging: '代理历史记录',
    proxyHistoryLoggingDesc: '配置代理历史记录设置',
    stopLoggingOutOfScope: '停止记录范围外的项目',
    askUser: '询问用户',
    doNothing: '无操作',
    defaultInterceptionState: '默认拦截状态',
    defaultInterceptionStateDesc: '配置请求和响应的默认拦截状态',
    enableInterception: '启用拦截',
    disableInterception: '禁用拦截',
    restoreInterceptionState: '恢复拦截状态',
    miscellaneousSettings: '其他设置',
    miscellaneousSettingsDesc: '配置代理的其他设置',
    useHTTP1_1ToServer: '对服务器使用 HTTP/1.1',
    useHTTP1_1ToClient: '对客户端使用 HTTP/1.1',
    setConnectionClose: '设置 Connection: Close',
    setConnectionHeader: '设置 Connection 头',
    stripProxyHeaders: '移除代理头',
    removeUnsupportedEncodings: '移除不支持的编码',
    stripWebSocketExtensions: '移除 WebSocket 扩展',
    unpackCompressedRequests: '解压缩请求',
    unpackCompressedResponses: '解压缩响应',
    suppressBurpErrorMessages: '抑制 Burp 错误消息',
    dontSendToProxyHistory: '不发送到代理历史记录',
    dontSendToProxyHistoryIfOutOfScope: '如果超出范围则不发送到代理历史记录',
    resetToDefaults: '重置为默认值'
  },
  // Proxifier Panel Component
  proxifier: {
    title: 'Proxifier',
    status: {
      running: '运行中',
      stopped: '已停止'
    },
    buttons: {
      start: '启动',
      stop: '停止'
    },
    tabs: {
      proxies: '代理服务器',
      rules: '规则',
      system: '系统'
    }
  },
  // Proxifier Proxies Component
  proxifierProxies: {
    title: '代理服务器',
    table: {
      name: '名称',
      port: '端口',
      type: '类型'
    },
    emptyState: {
      noProxies: '暂无代理服务器'
    },
    buttons: {
      add: '添加...',
      edit: '编辑...',
      remove: '删除',
      proxyChains: '代理链...'
    },
    description: '可以链接多个代理服务器：',
    dialog: {
      addTitle: '添加代理',
      editTitle: '编辑代理',
      host: {
        label: '主机地址',
        placeholder: '127.0.0.1 或 proxy.example.com'
      },
      port: {
        label: '端口',
        placeholder: '8080'
      },
      type: {
        label: '代理类型',
        http: 'HTTP',
        https: 'HTTPS',
        socks5: 'SOCKS5'
      },
      auth: '身份验证（可选）',
      username: {
        label: '用户名',
        placeholder: '可选'
      },
      password: {
        label: '密码',
        placeholder: '可选'
      },
      buttons: {
        cancel: '取消',
        save: '保存'
      }
    }
  },
  // Proxifier Proxies Component
  proxies: {
    title: '代理服务器',
    table: {
      name: '名称',
      port: '端口',
      type: '类型',
      noProxies: '暂无代理服务器'
    },
    buttons: {
      add: '添加...',
      edit: '编辑',
      delete: '删除'
    }
  },
  // Proxifier Panel Component
  proxifierPanel: {
    statusRunning: '运行中',
    statusStopped: '已停止',
    start: '启动',
    stop: '停止',
    application: '应用',
    target: '目标',
    timeOrStatus: '时间/状态',
    ruleProxy: '规则/代理',
    sent: '发送',
    received: '接收',
    noConnections: '暂无连接',
    startProxifierToShow: '启动 Proxifier 以显示连接',
    noLogs: '暂无日志',
    transparentProxy: '透明代理',
    transparentProxyStatus: '透明代理状态',
    status: '状态',
    running: '运行中',
    stopped: '已停止',
    enabled: '已启用',
    disabled: '已禁用',
    proxyPort: '代理端口',
    redirectPorts: '重定向端口',
    startTransparentProxy: '启动透明代理',
    stopTransparentProxy: '停止透明代理',
    transparentProxyDesc: '透明地拦截所有应用的流量',
    startTransparentProxyDesc: '启动透明代理以自动拦截所有应用的流量',
    stopTransparentProxyDesc: '停止透明代理以禁用自动流量拦截',
    pfFirewall: 'pf 防火墙',
    rules: {
      title: '代理规则',
      noRules: '暂无规则',
      table: {
        name: '名称',
        applications: '应用',
        applicationsExample: '例如：Safari、Chrome、Firefox',
        targetHosts: '目标主机',
        targetHostsExample: '例如：*.example.com、192.168.1.*',
        targetPorts: '目标端口',
        targetPortsExample: '例如：80、443、8080-8090',
        action: '动作',
        direct: '直连',
        block: '阻止',
        viaProxy: '通过代理',
        proxyFormat: '{type} {host}:{port}'
      }
    },
    buttons: {
      add: '添加',
      clone: '克隆',
      edit: '编辑',
      remove: '删除',
      enabled: '已启用',
      cancel: '取消',
      save: '保存',
      close: '关闭'
    }
  },
  // Proxifier Rules Component
  rules: {
    title: '代理规则',
    table: {
      select: '选择',
      name: '名称',
      applications: '应用',
      targetHosts: '目标主机',
      port: '端口',
      action: '动作',
      noRules: '暂无规则'
    }
  },
  workflowStudio: {
    title: '工作流工作室',
    header: {
      namePlaceholder: '工作流名称',
      editMetadataTooltip: '编辑工作流元数据'
    },
    toolbar: {
      load: '加载',
      loadTooltip: '加载工作流',
      templates: '模板',
      templateMarketTooltip: '模板市场',
      save: '保存',
      saveTooltip: '保存工作流',
      exportImportTooltip: '导出/导入',
      refreshCatalog: '刷新节点库',
      refreshCatalogTooltip: '刷新节点库',
      resetCanvas: '重置画布',
      resetCanvasTooltip: '重置画布',
      run: '运行',
      runTooltip: '运行工作流',
      stop: '停止',
      stopTooltip: '停止工作流',
      schedule: '定时',
      startScheduleTooltip: '启动定时调度',
      startScheduleDisabledTooltip: '请先保存工作流并添加定时触发节点',
      stopScheduleTooltip: '停止定时调度',
      logs: '日志',
      toggleLogsTooltip: '切换日志面板',
      history: '历史',
      executionHistoryTooltip: '执行历史'
    },
    export: {
      exportJson: '导出为JSON',
      importJson: '从JSON导入',
      exportImage: '导出为图片',
      exportedBy: 'Sentinel AI 工作流工作室'
    },
    sidebar: {
      nodeLibrary: '节点库',
      expandSidebar: '展开侧边栏',
      collapseSidebar: '折叠侧边栏',
      searchPlaceholder: '搜索节点...',
      clearSearchTooltip: '清空搜索',
      searchInCanvasTooltip: '在画布中搜索',
      favoritesOnly: '仅显示收藏',
      noMatchingNodes: '未找到匹配的节点',
      favorite: '收藏',
      unfavorite: '取消收藏'
    },
    logs: {
      title: '执行日志',
      clear: '清空',
      empty: '暂无日志',
      expandDetails: '展开详情',
      collapseDetails: '收起详情',
      executionId: '执行ID: {id}',
      newWorkflowCreated: '已新建工作流',
      validationFailed: '工作流校验失败: {message}',
      validationError: '校验出错: {error}',
      workflowExecutionStarted: '开始执行工作流: {name}',
      workflowStarted: '工作流已启动',
      startFailed: '启动失败: {error}',
      stoppingWorkflow: '正在停止工作流...',
      workflowStopped: '工作流已停止',
      stopFailed: '停止失败: {error}',
      scheduleStarted: '定时调度已启动: {desc}',
      scheduleStartFailed: '启动定时调度失败: {error}',
      scheduleStopped: '定时调度已停止',
      scheduleStopFailed: '停止定时调度失败: {error}',
      workflowSaved: '工作流已保存: {name}',
      workflowSavedAsTool: '工作流已保存: {name} (已设为工具)',
      saveFailed: '保存失败: {error}',
      workflowLoaded: '工作流已加载: {name}',
      loadFailed: '加载失败: {error}',
      workflowExported: '工作流已导出: {filename}',
      exportFailed: '导出失败: {error}',
      workflowImported: '工作流已导入: {name}',
      importFailed: '导入失败: {message}',
      imageExportTodo: '图片导出功能待实现',
      workflowExecutionStartedExternal: '工作流执行开始 (外部触发)',
      nodeStarted: '节点开始执行',
      nodeCompleted: '节点执行完成',
      workflowCompleted: '工作流执行完成',
      workflowExecutionStopped: '工作流执行已停止',
      foundMatchingNodes: '找到 {count} 个匹配的节点',
      noMatchingNodes: '未找到匹配的节点',
      templateSaved: '已保存为模板: {name}',
      templateSaveFailed: '保存模板失败: {error}'
    },
    loadDialog: {
      title: '加载工作流',
      empty: '暂无已保存的工作流',
      version: '版本: {version}',
      updated: '更新: {date}',
      deleteTooltip: '删除',
      close: '关闭'
    },
    templateMarket: {
      title: '工作流模板市场',
      recommended: '推荐模板',
      myTemplates: '我的模板',
      empty: '暂无模板',
      templateBadge: '模板',
      nodeCount: '{count} 个节点',
      useTemplate: '使用模板',
      saveAsTemplate: '另存为模板',
      saveCurrentAsTemplate: '保存当前为模板',
      close: '关闭'
    },
    newWorkflowConfirm: {
      title: '新建工作流',
      message: '当前工作流尚未保存，是否保存后再新建？',
      saveAndNew: '保存并新建',
      discardAndNew: '直接新建',
      cancel: '取消',
      close: '关闭'
    },
    metaDialog: {
      title: '工作流元数据',
      name: '工作流名称',
      namePlaceholder: '请输入工作流名称',
      description: '描述',
      descriptionPlaceholder: '描述工作流的用途和功能',
      tags: '标签',
      tagsPlaceholder: '用逗号分隔多个标签，如：自动化,数据处理',
      version: '版本',
      asAiTool: '设为AI工具',
      asAiToolHelp: '启用后，此工作流可作为AI助手的工具被调用',
      stats: {
        nodes: '节点数',
        edges: '连接数'
      },
      confirm: '确定',
      cancel: '取消'
    },
    paramsEditor: {
      title: '参数编辑',
      noParams: '此节点无需配置参数',
      selectNotificationRule: '-- 请选择通知规则 --',
      noNotificationRules: '⚠️ 暂无可用的通知规则，',
      goToConfigure: '前往配置',
      useDefaultConfig: '-- 使用默认配置 --',
      noAiProviders: '⚠️ 暂无可用的 AI 提供商，',
      selectModel: '请选择模型',
      selectProviderFirst: '请先选择提供商',
      noTools: '暂无可用工具',
      selectedToolsCount: '已选择 {count} 个工具',
      enterField: '请输入{key}',
      onePerLine: '每行输入一个值',
      pleaseSelect: '-- 请选择 --',
      booleanYes: '是',
      booleanNo: '否',
      arrayPlaceholder: '每行一个值，例如：\nhttps://example1.com/\nhttps://example2.com/',
      defaultValue: '默认: {value}',
      save: '保存',
      cancel: '取消'
    },
    executionHistory: {
      title: '执行历史',
      clear: '清空',
      clearTooltip: '清空历史',
      emptyTitle: '暂无执行记录',
      emptyDescription: '运行工作流后会在此显示历史',
      status: {
        completed: '✓ 完成',
        failed: '✗ 失败',
        running: '● 运行中',
        pending: '○ 等待'
      },
      deleteRecordTooltip: '删除此记录',
      durationMs: '耗时: {ms}ms',
      detailsTitle: '执行详情',
      copyResultsTooltip: '复制结果'
    },
    resultPanel: {
      title: '步骤执行结果',
      copyTooltip: '复制结果',
      nodeId: '节点 ID',
      nodeName: '节点名称',
      unknown: '未知',
      executionResult: '执行结果',
      editParams: '编辑参数',
      close: '关闭',
      noResult: '暂无结果'
    },
    groups: {
      trigger: '触发器',
      control: '控制流',
      ai: 'AI',
      data: '数据',
      output: '输出/通知',
      tool: '内置工具',
      mcp: 'MCP工具',
      plugin: 'Agent插件'
    },
    schedule: {
      everySeconds: '每 {seconds} 秒',
      dailyAt: '每天 {time}',
      weeklyAt: '每周 {weekdays} {time}'
    },
    confirm: {
      deleteWorkflow: '确定要删除这个工作流吗？'
    },
    toasts: {
      enterWorkflowName: '请先输入工作流名称',
      newWorkflowCreated: '已新建工作流',
      copiedToClipboard: '结果已复制到剪贴板',
      copyFailed: '复制失败：{message}',
      validationFailed: '校验失败：{message}',
      validationError: '校验出错：{error}',
      executionStarted: '已启动执行：{id}',
      startFailed: '启动失败：{error}',
      noRunningWorkflow: '没有正在运行的工作流',
      workflowStopped: '工作流已停止',
      stopFailed: '停止失败：{error}',
      scheduleMissingTrigger: '请先添加定时触发节点并配置参数',
      scheduleStarted: '定时调度已启动: {desc}',
      scheduleStartFailed: '启动定时调度失败: {error}',
      scheduleStopped: '定时调度已停止',
      scheduleStopFailed: '停止定时调度失败: {error}',
      workflowSaved: '工作流已保存',
      saveFailed: '保存失败：{error}',
      loadFailed: '加载失败：{error}',
      workflowDeleted: '工作流已删除',
      deleteFailed: '删除失败：{error}',
      workflowExported: '工作流已导出',
      exportFailed: '导出失败：{error}',
      workflowImported: '工作流已导入',
      importFailed: '导入失败：{message}',
      imageExportRequiresHtml2Canvas: '图片导出功能需要安装html2canvas库',
      templateSaved: '已保存为模板',
      templateSaveFailed: '保存模板失败：{error}'
    },
    flowchart: {
      toolbar: {
        title: '执行流程图',
        newWorkflow: '新建',
        newWorkflowTooltip: '新建工作流',
        aiGenerate: 'AI生成',
        aiGenerateTooltip: '通过自然语言生成工作流',
        resetView: '重置视图',
        arrangeNodes: '整理节点',
        arrangeNodesTooltip: '自动整理节点布局',
        undoTooltip: '撤销 (Ctrl+Z)',
        redoTooltip: '重做 (Ctrl+Y)',
        deleteConnection: '删除连接',
        deleteConnectionTooltip: '点击连接线删除',
        exitFullscreen: '退出全屏'
      },
      emptyState: {
        title: '画布为空',
        description: '从左侧节点库拖拽节点到这里开始创建工作流',
        tip: '提示：按住 Shift 键拖拽可以平移画布'
      },
      ports: {
        input: '输入',
        output: '输出'
      },
      breakpoints: {
        title: '断点'
      },
      status: {
        pending: '待执行',
        planning: '规划中',
        running: '执行中',
        completed: '已完成',
        failed: '失败',
        paused: '已暂停',
        cancelled: '已取消'
      },
      contextMenu: {
        addBreakpoint: '添加断点',
        removeBreakpoint: '移除断点',
        duplicateNode: '复制节点',
        deleteNode: '删除节点',
        duplicateNodeName: '{name} (副本)'
      },
      aiGenerate: {
        title: 'AI生成工作流',
        help: '用自然语言描述你想要的流程，例如：先子域名扫描，再端口扫描，最后用AI分析结果并生成报告。',
        placeholder: '请输入工作流描述...',
        cancel: '取消',
        generateAndLoad: '生成并加载',
        missingNodesError: '生成结果缺少 nodes'
      }
    },
    defaults: {
      unnamedWorkflow: '未命名工作流',
      importedWorkflow: '导入的工作流',
      duplicateWorkflowName: '{name} (副本)'
    },
    errors: {
      invalidWorkflowFile: '无效的工作流文件格式',
      jsonFormatError: 'JSON格式错误: {message}'
    }
  }
}
