export default {
    title: 'Notification Management',

    // Buttons and actions
    newNotification: 'New Notification',
    editNotification: 'Edit Notification',
    edit: 'Edit',
    delete: 'Delete',
    cancel: 'Cancel',
    confirm: 'Confirm',
    testConnection: 'Test Connection',

    // Statistics
    totalCount: 'Total {count} items',

    // Table columns
    table: {
        notificationType: 'Notification Type',
        notificationStatus: 'Notification Status',
        notificationConfig: 'Notification Config',
        updateTime: 'Update Time',
        actions: 'Actions',
    },

    // Form
    form: {
        notificationType: 'Notification Type',
        typeNamePlaceholder: 'Enter notification type',
        description: 'Description',
        descriptionPlaceholder: 'Optional, additional notes',
        webhookUrl: 'WebHook URL',
        secret: 'Secret',
        remarks: 'User Remarks',
    },

    // Channels
    channels: {
        feishu: 'Feishu',
        dingtalk: 'DingTalk',
        wecom: 'WeCom',
        webhook: 'Webhook',
        email: 'Email',
    },

    // Email configuration
    email: {
        smtpServer: 'SMTP Server',
        smtpServerPlaceholder: 'Hostname, domain or IP address',
        port: 'Port',
        portPlaceholder: '25',
        transportEncryption: 'Transport Encryption',
        tls: 'TLS',
        ssl: 'SSL',
        none: 'Unencrypted (Plain text)',
        senderAccount: 'Sender Email Account',
        senderAccountPlaceholder: 'Leave empty to not use account',
        senderPassword: 'Sender Email Password',
        senderPasswordPlaceholder: 'Leave empty to use empty password',
        senderAddress: 'Sender Email Address',
        senderAddressPlaceholder: 'Leave empty to use default address',
        recipientAddress: 'Recipient Address',
        recipientAddressPlaceholder: 'Separate multiple addresses with commas',
    },

    // Tips
    tips: {
        ensureRequiredInfo: '1. Ensure all required information above is complete',
    },

    // Messages
    messages: {
        noRules: 'No rules',
        deleteConfirm: 'Are you sure you want to delete this notification?',
        deleteTitle: 'Delete Notification',
        webhookUrlRequired: 'Please enter WebHook URL',
        connectionNormal: 'Connection normal',
        connectionFailed: 'Connection failed',
        testTriggered: 'Test triggered',
        testSent: 'Test sent',
        testSimulated: 'Send failed',
    },

    // Pagination
    perPage: 'Per page',
}
