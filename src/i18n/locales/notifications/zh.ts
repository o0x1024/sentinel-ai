export default {
    title: '通知管理',

    // 按钮和操作
    newNotification: '新建通知',
    editNotification: '编辑通知',
    edit: '编辑',
    delete: '删除',
    cancel: '取消',
    confirm: '确定',
    testConnection: '测试连接状态',

    // 统计
    totalCount: '共 {count} 条',

    // 表格列
    table: {
        notificationType: '通知类型',
        notificationStatus: '通知状态',
        notificationConfig: '通知配置',
        updateTime: '更新时间',
        actions: '操作',
    },

    // 表单
    form: {
        notificationType: '通知类型',
        typeNamePlaceholder: '请输入通知类型',
        description: '说明',
        descriptionPlaceholder: '可选，补充说明',
        webhookUrl: 'WebHookURL',
        secret: 'Secret',
        remarks: '用户备注',
    },

    // 通道
    channels: {
        feishu: '飞书',
        dingtalk: '钉钉',
        wecom: '企业微信',
        webhook: 'Webhook',
        email: '邮件',
    },

    // 邮件配置
    email: {
        smtpServer: 'SMTP 服务器',
        smtpServerPlaceholder: '主机名、域名或 IP 地址',
        port: '端口',
        portPlaceholder: '25',
        transportEncryption: '传输加密方式',
        tls: 'TLS',
        ssl: 'SSL',
        none: '未加密（明文传输）',
        senderAccount: '发件邮箱账号',
        senderAccountPlaceholder: '为空时不使用账号',
        senderPassword: '发件邮箱密码',
        senderPasswordPlaceholder: '为空时使用空密码',
        senderAddress: '发件邮箱地址',
        senderAddressPlaceholder: '为空时使用默认地址',
        recipientAddress: '收件人地址',
        recipientAddressPlaceholder: '多个地址用逗号分隔',
    },

    // 提示信息
    tips: {
        ensureRequiredInfo: '1. 确保上方必填信息完整',
    },

    // 消息
    messages: {
        noRules: '暂无规则',
        deleteConfirm: '确定删除该通知吗?',
        deleteTitle: '删除通知',
        webhookUrlRequired: '请填写WebHookURL',
        connectionNormal: '连接正常',
        connectionFailed: '连接失败',
        testTriggered: '已触发测试',
        testSent: '测试已发送',
        testSimulated: '发送失败',
    },

    // 分页
    perPage: '每页',
}
