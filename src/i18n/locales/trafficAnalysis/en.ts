export default {
  title: 'Traffic Analysis',
  tabs: {
    control: 'Proxy Control',
    history: 'History',
    repeater: 'Repeater',
    proxifier: 'Proxifier',
    capture: 'Capture',
    proxyConfig: 'Proxy Configuration'
  },
  ariaLabels: {
    trafficAnalysisTabs: 'Traffic analysis tabs'
  },
  tooltips: {
    control: 'Proxy Control',
    history: 'History Records',
    repeater: 'Repeater',
    proxifier: 'Proxifier',
    capture: 'Capture',
    proxyConfig: 'Proxy Configuration'
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
    title: 'Intercept',
    status: {
      on: 'Intercept is on',
      off: 'Intercept is off'
    },
    buttons: {
      turnOn: 'Turn on',
      turnOff: 'Turn off',
      toggleHttp: 'Toggle HTTP intercept',
      toggleWs: 'Toggle WebSocket intercept',
      forward: 'Forward',
      drop: 'Drop',
      forwardAll: 'Forward All',
      dropAll: 'Drop All',
      save: 'Save',
      edit: 'Edit',
      sendToRepeater: 'Send to Repeater'
    },
    stats: {
      proxyStatus: 'Proxy Status',
      running: 'Running',
      stopped: 'Stopped',
      port: 'Port',
      interceptQueue: 'Intercept Queue'
    },
    tabs: {
      raw: 'Raw',
      pretty: 'Pretty',
      hex: 'Hex'
    },
    requestLine: 'Request Line',
    statusLine: 'Status Line',
    headers: 'Headers',
    body: 'Body',
    waiting: 'Waiting for intercepted requests/responses...',
    enabled: 'Intercept is currently enabled',
    disabled: 'Please enable intercept functionality',
    proxyConfig: 'Configure browser proxy to',
    queue: 'Intercept Queue',
    response: 'Response',
    clickToView: 'Click on a request in the queue above to view details',
    request: 'Request',
    sentToAssistant: 'Sent {type} to AI Assistant',
    tooltip: {
      http: 'HTTP intercept status',
      websocket: 'WebSocket intercept status'
    },
    contextMenu: {
      sendToRepeater: 'Send to Repeater',
      sendToAI: 'Send to AI Assistant',
      addFilter: 'Add Filter',
      filterByDomain: 'Filter by Domain',
      filterByUrl: 'Filter by URL',
      filterByMethod: 'Filter by Method',
      filterByFileExt: 'Filter by File Extension',
      filterByStatus: 'Filter by Status Code',
      filterByContentType: 'Filter by Content-Type',
      filterByDirection: 'Filter by Direction',
      customFilter: 'Custom Filter...',
      copyUrl: 'Copy URL',
      copyAsCurl: 'Copy as cURL',
      copyRaw: 'Copy Raw Content'
    },
    filterDialog: {
      title: 'Add Intercept Filter Rule',
      filterType: 'Filter Type',
      typeRequest: 'Request',
      typeResponse: 'Response',
      matchType: 'Match Type',
      matchDomain: 'Domain',
      matchUrl: 'URL',
      matchMethod: 'HTTP Method',
      matchFileExt: 'File Extension',
      matchHeader: 'Header',
      matchStatus: 'Status Code',
      matchContentType: 'Content-Type',
      relationship: 'Relationship',
      matches: 'Matches',
      notMatches: 'Does not match',
      contains: 'Contains',
      notContains: 'Does not contain',
      condition: 'Condition',
      conditionPlaceholder: 'Enter value to match...',
      action: 'Action',
      actionExclude: 'Exclude from interception',
      actionInclude: 'Include in interception',
      cancel: 'Cancel',
      save: 'Save Rule',
      ruleAdded: 'Filter rule added successfully',
      addFailed: 'Failed to add filter rule'
    },
    aiDialog: {
      title: 'AI Security Assistant',
      customPrompt: 'Custom Prompt',
      promptPlaceholder: 'Ask AI anything about this request/response...',
      response: 'AI Response',
      processing: 'AI is analyzing...',
      close: 'Close',
      send: 'Send',
      analyze: 'Analyze Security',
      explain: 'Explain Request',
      suggest: 'Suggest Payloads',
      decode: 'Decode Parameters'
    }
  },
  // Proxy Control Component
  control: {
    title: 'Proxy Control',
    startProxy: 'Start Proxy',
    stopProxy: 'Stop Proxy',
    processing: 'Processing...',
    refreshStatus: 'Refresh Status',
    proxyConfig: 'Configure browser proxy to',
    proxySettings: 'Proxy configuration and interception rules can be set in the Proxy Settings page',
    stats: {
      proxyStatus: 'Proxy Status',
      running: 'Running',
      stopped: 'Stopped',
      notStarted: 'Not started',
      mitmStatus: 'MITM Status',
      enabled: 'Enabled',
      disabled: 'Disabled',
      mitmDesc: 'Man-in-the-middle interception',
      qps: 'QPS',
      qpsDesc: 'Queries per second',
      requestStats: 'Request Statistics',
      http: 'HTTP',
      https: 'HTTPS'
    }
  },
  // Proxy History Component
  history: {
    title: 'Request History',
    emptyState: {
      noRequests: 'No request history',
      selectInterface: 'Please select a network interface'
    },
    websocket: {
      toServer: 'To Server',
      fromServer: 'From Server'
    },
    contextMenu: {
      sendToRepeater: 'Send to Repeater',
      sendRequestToAssistant: 'Send Request to Assistant',
      sendResponseToAssistant: 'Send Response to Assistant',
      copyUrl: 'Copy URL',
      copyAsCurl: 'Copy as cURL',
      copyRequest: 'Copy Request',
      openInBrowser: 'Open in Browser',
      clearHistory: 'Clear History',
      addToFilter: 'Add to Filter',
      filterByDomain: 'Domain',
      filterByUrl: 'URL',
      filterByMethod: 'Method',
      filterByExtension: 'Extension'
    },
    certificateError: {
      title: 'Certificate Error',
      message: 'The certificate for this website is non-standard or has issues',
      details: 'Certificate Details',
      commonIssues: {
        invalidCN: 'Invalid CN (Common Name) format',
        expired: 'Certificate expired',
        selfSigned: 'Self-signed certificate',
        untrusted: 'Certificate chain not trusted',
        hostnameMMismatch: 'Hostname mismatch',
        weakSignature: 'Weak signature algorithm'
      },
      actions: {
        trustCert: 'Trust Certificate',
        viewDetails: 'View Details',
        ignore: 'Continue',
        stop: 'Stop Capture'
      },
      tips: {
        installCA: 'Make sure Sentinel AI root CA is installed and trusted',
        checkCAInstallation: 'Check Certificate Installation',
        caNotTrusted: 'Root CA not trusted',
        serverCertIssue: 'Target server certificate has issues'
      }
    },
    errors: {
      loadFailed: 'Failed to load request history',
      loadMoreFailed: 'Failed to load more',
      clearFailed: 'Failed to clear history',
      wsConnectionFailed: 'Failed to load WebSocket connections',
      wsMessagesFailed: 'Failed to load WebSocket messages',
      exportFailed: 'Export failed',
      networkError: 'Network error',
      timeout: 'Request timeout',
      unknown: 'Unknown error'
    },
    messages: {
      historyCleared: 'History cleared',
      exportSuccess: 'Export successful',
      copiedToClipboard: 'Copied to clipboard',
      sentToRepeater: 'Sent to Repeater',
      sentToAssistant: 'Sent to AI Assistant'
    },
    filterDialog: {
      title: 'Filter Settings',
      search: 'Search',
      method: 'Method',
      host: 'Host',
      url: 'URL',
      statusCode: 'Status Code',
      contentType: 'Content Type',
      contains: 'Contains',
      startTime: 'Start Time',
      endTime: 'End Time',
      reset: 'Reset',
      apply: 'Apply',
      checkboxes: {
        showOnlyWithParams: 'Show only parameterized requests',
        hideWithoutResponse: 'Hide items without responses',
        html: 'HTML',
        javascript: 'JavaScript',
        css: 'CSS',
        json: 'JSON',
        xml: 'XML',
        image: 'Image',
        other: 'Other',
        otherText: 'Other text',
        script: 'Script',
        images: 'Images',
        flash: 'Flash',
        otherBinary: 'Other binary',
        port: 'Port',
        regex: 'Regex',
        caseSensitive: 'Case sensitive',
        negativeSearch: 'Negative search',
        placeholders: {
          port: 'e.g. 8080',
          search: 'Search...',
          showExtensions: 'asp,aspx,jsp,php',
          hideExtensions: 'js,gif,jpg,png,css'
        },
        labels: {
          showOnly: 'Show only:',
          hide: 'Hide:'
        }
      }
    },
    table: {
      id: 'ID',
      time: 'Time',
      method: 'Method',
      host: 'Host',
      url: 'URL',
      status: 'Status',
      length: 'Length',
      mimeType: 'MIME Type',
      actions: 'Actions'
    },
    detailsPanel: {
      requestDetails: 'Request Details',
      responseDetails: 'Response Details',
      tabs: {
        headers: 'Headers',
        raw: 'Raw',
        pretty: 'Pretty',
        hex: 'Hex',
        render: 'Render'
      },
      request: 'Request',
      response: 'Response',
      decompressed: 'Decompressed',
      close: 'close',
      originalRequest: 'Original request',
      editedRequest: 'Edited request',
      originalResponse: 'Original response',
      editedResponse: 'Edited response'
    },
    // ProxifierProxies Component
    proxifierProxies: {
      title: 'Proxy Servers',
      table: {
        name: 'Name',
        port: 'Port',
        type: 'Type'
      },
      emptyState: {
        noProxies: 'No proxy servers'
      },
      buttons: {
        add: 'Add...',
        edit: 'Edit...',
        remove: 'Remove',
        proxyChains: 'Proxy Chains...'
      },
      description: 'Multiple proxy servers can be chained:',
      dialog: {
        addTitle: 'Add Proxy',
        editTitle: 'Edit Proxy',
        host: {
          label: 'Host Address',
          placeholder: '127.0.0.1 or proxy.example.com'
        },
        port: {
          label: 'Port',
          placeholder: '8080'
        },
        type: {
          label: 'Proxy Type',
          http: 'HTTP',
          https: 'HTTPS',
          socks5: 'SOCKS5'
        },
        auth: 'Authentication (Optional)',
        username: {
          label: 'Username',
          placeholder: 'Optional'
        },
        password: {
          label: 'Password',
          placeholder: 'Optional'
        },
        buttons: {
          cancel: 'Cancel',
          save: 'Save'
        }
      }
    },
    toolbar: {
      filter: 'Filter',
      clear: 'Clear',
      refresh: 'Refresh',
      export: 'Export',
      searchPlaceholder: 'Search requests...'
    },
    export: {
      sendToAssistant: 'Send to AI Assistant',
      exportToFile: 'Export to File',
      exportRequest: 'Export Request',
      exportResponse: 'Export Response',
      exportHAR: 'Export as HAR',
      exportAsHAR: 'Export as HAR',
      request: 'request',
      response: 'response',
      noSelection: 'Please select requests to export first',
      success: 'Successfully exported {count} {type}(s) to file',
      harSuccess: 'Successfully exported {count} requests as HAR format',
      failed: 'Export failed: {error}',
      saveHistory: 'Save History',
      loadHistory: 'Load History',
      historySaved: 'Saved {count} records to database',
      historyLoaded: 'Loaded {count} records from database',
      saveFailed: 'Save failed: {error}',
      loadFailed: 'Load failed: {error}',
      persistenceConfig: 'Persistence Configuration',
      enableAutoPersistence: 'Enable Auto Persistence',
      autoPersistenceThreshold: 'Auto Persistence Threshold',
      autoPersistenceInterval: 'Auto Persistence Interval (seconds)',
      thresholdHelp: 'Auto-save to database when cache reaches this count',
      intervalHelp: 'Periodic auto-save interval',
      compressionEnabled: 'Response Body Compression',
      compressionHelp: 'Auto-compress response bodies larger than 1KB to save storage',
      configSaved: 'Configuration saved',
      configSaveFailed: 'Failed to save configuration: {error}'
    }
  },
  // Proxy Repeater Component
  repeater: {
    contextMenu: {
      sendRequest: 'Send Request',
      sendToNewTab: 'Send to Repeater',
      copyUrl: 'Copy URL',
      copyRequest: 'Copy Request',
      copyAsCurl: 'Copy as cURL',
      sendRequestToAssistant: 'Send Request to Assistant',
      sendResponseToAssistant: 'Send Response to Assistant',
      paste: 'Paste',
      clear: 'Clear',
      close: 'Close',
      newTab: 'New Tab',
      horizontalLayout: 'Horizontal Layout',
      verticalLayout: 'Vertical Layout',
      cancel: 'Cancel',
      ok: 'OK',
      target: 'Target',
      configureTargetDetails: 'Configure target details',
      host: 'Host',
      overrideSni: 'Override SNI',
      sniHostname: 'SNI hostname',
      port: 'Port',
      useHttps: 'Use HTTPS',
      request: 'Request',
      response: 'Response',
      pretty: 'Pretty',
      raw: 'Raw',
      hex: 'Hex',
      render: 'Render',
      clickSendToSendRequest: 'Click "Send Request" to send the request'
    },
    tabContextMenu: {
      closeTab: 'Close Tab',
      closeOthers: 'Close Other Tabs',
      closeAll: 'Close All Tabs',
      closeLeft: 'Close Tabs to the Left',
      closeRight: 'Close Tabs to the Right'
    },
    messages: {
      requestCancelled: 'Request cancelled',
      fillTargetAndRequest: 'Please fill in target host and request content',
      urlCopied: 'URL copied',
      copyFailed: 'Copy failed',
      requestCopied: 'Request copied',
      curlCopied: 'cURL command copied',
      pasted: 'Pasted',
      cannotReadClipboard: 'Cannot read clipboard',
      sentToAssistant: 'Sent {type} to AI Assistant',
      noResponseData: 'No response data available',
      requestFailed: 'Request failed',
      sendRequestFailed: 'Failed to send request',
      networkError: 'Network error',
      timeout: 'Request timeout',
      connectionRefused: 'Connection refused',
      unknownError: 'Unknown error',
      hexDisplayLimited: 'Hex display limited to first {size}',
      invalidUrl: 'Invalid URL format',
      confirmCloseTab: 'Confirm close tab?',
      confirmCloseTabMessage: 'This tab has unsaved content. Are you sure you want to close it?',
      confirmCloseAllTabs: 'Confirm close all tabs?',
      confirmCloseAllTabsMessage: 'Some tabs have unsaved content. Are you sure you want to close all tabs?',
      tabRestored: 'Restored {count} tab(s)',
      tabsSaved: 'Tabs auto-saved',
      saveTabsFailed: 'Failed to save tabs'
    },
    types: {
      request: 'request',
      response: 'response',
      both: 'traffic'
    }
  },
  // Packet Capture Component
  capture: {
    toolbar: {
      interfaceSelect: 'Interface Selection',
      selectInterface: 'Select Interface',
      startStop: 'Start/Stop Button',
      start: 'Start',
      stop: 'Stop',
      clear: 'Clear',
      openFile: 'Open File',
      saveFile: 'Save File',
      extractFile: 'Extract File',
      advancedFilter: 'Advanced Filter',
      filterInput: 'Filter Input',
      statistics: 'Statistics',
      advancedFiltering: 'Advanced Filtering'
    },
    table: {
      time: 'Time',
      source: 'Source',
      destination: 'Destination',
      protocol: 'Protocol',
      length: 'Length',
      info: 'Info',
      no: 'No.'
    },
    statusBar: {
      capturing: 'Capturing',
      selected: 'Selected',
      captured: 'captured',
      packets: 'packets'
    },
    contextMenu: {
      mark: 'Mark',
      unmark: 'Unmark',
      ignore: 'Ignore',
      unignore: 'Unignore',
      filter: 'Filter',
      filterByField: 'Filter by Field',
      sourceAddress: 'Source Address',
      destinationAddress: 'Destination Address',
      protocol: 'Protocol',
      conversation: 'Conversation',
      followStream: 'Follow Stream',
      tcpStream: 'TCP Stream',
      udpStream: 'UDP Stream',
      httpStream: 'HTTP Stream',
      copy: 'Copy',
      summary: 'Summary',
      copySource: 'Source Address',
      copyDestination: 'Destination Address',
      filterByValue: 'Filter by this value'
    },
    emptyStates: {
      gettingInterfaces: 'Getting interface list...',
      noInterfaces: 'No available interfaces detected',
      installNpcap: 'Windows systems require Npcap driver for network packet capture',
      downloadNpcap: 'Download Npcap',
      selectAndStart: 'Select interface and click start capture',
      waitingForPackets: 'Waiting for packets...'
    },
    hexView: {
      hex: 'Hex',
      ascii: 'ASCII',
      raw: 'Raw'
    }
  },
  // Packet Capture Component (new structure)
  packetCapture: {
    toolbar: {
      selectInterface: 'Select Interface',
      start: 'Start',
      stop: 'Stop',
      clear: 'Clear',
      open: 'Open File',
      save: 'Save File',
      export: 'Export',
      advancedFilter: 'Advanced Filter',
      advancedFilterBadge: 'Advanced',
      filterPlaceholder: 'Filter packets...',
      filtering: 'Filtering'
    },
    table: {
      no: 'No.',
      time: 'Time',
      source: 'Source',
      destination: 'Destination',
      protocol: 'Protocol',
      length: 'Length',
      info: 'Info'
    },
    statusBar: {
      capturing: 'Capturing',
      selected: 'Selected',
      captured: 'captured',
      packets: 'packets'
    },
    emptyState: {
      loadingInterfaces: 'Getting interface list...',
      noInterfaces: 'No available interfaces detected',
      npcapRequired: 'Windows systems require Npcap driver for network packet capture',
      downloadNpcap: 'Download Npcap',
      selectAndStart: 'Select interface and click start capture',
      waitingForPackets: 'Waiting for packets...'
    },
    contextMenu: {
      mark: 'Mark',
      unmark: 'Unmark',
      ignore: 'Ignore',
      unignore: 'Unignore',
      filter: 'Filter',
      filterByField: 'Filter by Field',
      sourceAddress: 'Source Address',
      destinationAddress: 'Destination Address',
      protocol: 'Protocol',
      conversation: 'Conversation',
      followStream: 'Follow Stream',
      tcpStream: 'TCP Stream',
      udpStream: 'UDP Stream',
      httpStream: 'HTTP Stream',
      copy: 'Copy',
      summary: 'Summary',
      copySource: 'Source Address',
      copyDestination: 'Destination Address',
      filterThisValue: 'Filter by this value',
      hex: 'Hex'
    },
    filterDialog: {
      title: 'Advanced Filter',
      protocol: 'Protocol',
      sourceIp: 'Source IP',
      destinationIp: 'Destination IP',
      sourcePort: 'Source Port',
      destinationPort: 'Destination Port',
      containsString: 'Contains String',
      containsHex: 'Contains Hex',
      minLength: 'Min Length',
      maxLength: 'Max Length',
      tcpFlags: 'TCP Flags',
      reset: 'Reset',
      cancel: 'Cancel',
      apply: 'Apply'
    },
    streamDialog: {
      ascii: 'ASCII',
      hex: 'Hex',
      raw: 'Raw',
      clientToServer: 'Client → Server',
      serverToClient: 'Server → Client',
      packets: 'packets',
      close: 'Close'
    },
    extractDialog: {
      title: 'File Extraction',
      foundFiles: 'Found {count} files',
      analyzing: 'Analyzing packets for file extraction...',
      supportedProtocols: 'Supports: HTTP, FTP, Email attachments, DNS tunneling',
      noFilesFound: 'No files found in current packet selection',
      protocolExamples: 'Examples: HTTP downloads, FTP transfers, email attachments',
      filterConditions: 'Filter Conditions',
      clearFilter: 'Clear Filter',
      filename: 'Filename',
      searchFilename: 'Search filename...',
      fileType: 'File Type',
      allTypes: 'All Types',
      image: 'Image',
      video: 'Video',
      audio: 'Audio',
      archive: 'Archive',
      document: 'Document',
      executable: 'Executable',
      other: 'Other',
      sourceProtocol: 'Source Protocol',
      allSources: 'All Sources',
      fileSize: 'File Size',
      anySize: 'Any Size',
      sizeTiny: 'Tiny (< 1KB)',
      sizeSmall: 'Small (1KB - 10KB)',
      sizeMedium: 'Medium (10KB - 1MB)',
      sizeLarge: 'Large (1MB - 10MB)',
      sizeHuge: 'Huge (> 10MB)',
      http: 'HTTP',
      ftp: 'FTP',
      email: 'Email',
      dnsTunnel: 'DNS Tunnel',
      type: 'Type',
      size: 'Size',
      source: 'Source',
      traffic: 'Traffic',
      actions: 'Actions',
      downloadFile: 'Download File',
      traceTraffic: 'Trace Traffic',
      locatePackets: 'Locate Packets',
      close: 'Close',
      saveSelectedFiles: 'Save Selected Files',
      selectedFiles: 'Selected Files',
      selectedSize: 'Selected Size'
    }
  },
  // Proxy Configuration Component
  configuration: {
    title: 'Proxy Listeners',
    description: 'Configure proxy listeners to receive HTTP requests from browsers. You need to configure your browser to use one of these listeners as a proxy server.',
    table: {
      select: 'Select',
      running: 'Running',
      interface: 'Interface',
      invisible: 'Invisible',
      redirect: 'Redirect',
      certificate: 'Certificate',
      tlsProtocol: 'TLS Protocol',
      supportHttp2: 'Support HTTP/2'
    }
  },
  // Proxy Configuration Component (new structure)
  proxyConfiguration: {
    proxyListenersTitle: 'Proxy Listeners',
    proxyListenersDescription: 'Configure proxy listeners to receive HTTP requests from browsers. You need to configure your browser to use one of these listeners as a proxy server.',
    autoStartProxy: 'Auto-start proxy on application launch',
    autoStartProxyDesc: 'When enabled, the proxy listener will start automatically when the application launches, without requiring manual intervention',
    trafficAnalysisSettings: 'Traffic Analysis Settings',
    trafficAnalysisSettingsDesc: 'Configure traffic analysis scanning and filtering options',
    excludeSelfTraffic: 'Exclude self traffic from scanning',
    excludeSelfTrafficDesc: 'When enabled, HTTP requests from this application will not be scanned by traffic analysis plugins, but will still appear in traffic history',
    enableTrafficAnalysisPlugin: 'Enable traffic analysis plugin scanning',
    enableTrafficAnalysisPluginDesc: 'When enabled, traffic analysis plugins will automatically scan captured traffic for potential security vulnerabilities',
    running: 'Running',
    interface: 'Interface',
    invisible: 'Invisible',
    redirect: 'Redirect',
    certificate: 'Certificate',
    tlsProtocols: 'TLS Protocols',
    http2Support: 'Support HTTP/2',
    addListener: 'Add Listener',
    editListener: 'Edit Listener',
    removeListener: 'Remove Listener',
    exportCACert: 'Import / export CA certificate',
    regenerateCACert: 'Regenerate CA Certificate',
    openCertDir: 'Open Certificate Directory',
    certInfo: 'Certificate Info',
    // CA Certificate Dialog
    caCertDialogTitle: 'CA Certificate',
    caCertDialogDesc: 'You can export your certificate and key for use in other tools, or in another installation. You can import a certificate and key to use in this installation. Note that you can also export the current certificate by visiting http://burpsuite/cert in your browser.',
    exportSection: 'Export',
    importSection: 'Import',
    certInDerFormat: 'Certificate in DER format',
    privateKeyInDerFormat: 'Private key in DER format',
    certAndKeyInPkcs12: 'Certificate and private key in PKCS#12 keystore',
    certAndKeyInDerFormat: 'Certificate and private key in DER format',
    certAndKeyFromPkcs12: 'Certificate and private key from PKCS#12 keystore',
    next: 'Next',
    bindAddress: 'Bind Address',
    port: 'Port',
    certMode: 'Certificate Mode',
    perHostCert: 'Per-host Certificate',
    wildcardCert: 'Wildcard Certificate',
    customCert: 'Custom Certificate',
    defaultTLS: 'Default',
    tls12: 'TLS 1.2',
    tls13: 'TLS 1.3',
    tls12Plus13: 'TLS 1.2 + 1.3',
    supportHTTP2: 'Support HTTP/2',
    invisibleMode: 'Invisible Mode',
    enableRedirect: 'Enable Redirect',
    cancel: 'Cancel',
    save: 'Save',
    close: 'Close',
    interceptionRules: 'Interception Rules',
    interceptionRulesDesc: 'Configure interception rules for requests and responses',
    requestInterceptionRules: 'Request Interception Rules',
    requestInterceptionRulesDesc: 'Use these settings to control which requests are stalled for viewing and editing in the Intercept tab.',
    responseInterceptionRules: 'Response Interception Rules',
    responseInterceptionRulesDesc: 'Use these settings to control which responses are stalled for viewing and editing in the Intercept tab.',
    interceptRequests: 'Intercept requests based on the following rules:',
    interceptResponses: 'Intercept responses based on the following rules:',
    masterInterceptionDisabled: 'Master interception is turned off',
    enable: 'Enabled',
    operator: 'Operator',
    matchType: 'Match type',
    relationship: 'Relationship',
    condition: 'Condition',
    addRule: 'Add',
    editRule: 'Edit',
    removeRule: 'Remove',
    moveUp: 'Up',
    moveDown: 'Down',
    autoFixNewlines: 'Automatically fix missing or superfluous new lines at end of request',
    autoUpdateContentLength: 'Automatically update Content-Length header when the request is edited',
    autoUpdateResponseContentLength: 'Automatically update Content-Length header when the response is edited',
    // Rule edit dialog
    addInterceptionRule: 'Add interception rule',
    editInterceptionRule: 'Edit interception rule',
    specifyRuleDetails: 'Specify the details of the interception rule.',
    booleanOperator: 'Boolean operator:',
    matchRelationship: 'Match relationship:',
    matchCondition: 'Match condition:',
    conditionPlaceholder: 'Enter regex pattern or value',
    ok: 'OK',
    // Match types
    matchTypes: {
      domain_name: 'Domain name',
      ip_address: 'IP address',
      protocol: 'Protocol',
      http_method: 'HTTP method',
      url: 'URL',
      file_extension: 'File extension',
      request: 'Request',
      cookie_name: 'Cookie name',
      cookie_value: 'Cookie value',
      any_header: 'Any header',
      body: 'Body',
      param_name: 'Param name',
      param_value: 'Param value',
      listener_port: 'Listener port',
      status_code: 'Status code',
      content_type_header: 'Content type header'
    },
    // Relationships
    relationships: {
      matches: 'Matches',
      does_not_match: 'Does not match',
      contains_parameters: 'Contains parameters',
      is_in_target_scope: 'Is in target scope',
      was_modified: 'Was modified',
      was_intercepted: 'Was intercepted'
    },
    websocketInterceptionRules: 'WebSocket Interception Rules',
    websocketInterceptionRulesDesc: 'Configure WebSocket interception rules',
    interceptClientToServer: 'Intercept Client → Server',
    interceptServerToClient: 'Intercept Server → Client',
    onlyInterceptInScope: 'Only intercept in-scope items',
    responseModificationRules: 'Response Modification Rules',
    responseModificationRulesDesc: 'Configure response modification rules',
    unhideHiddenFields: 'Unhide hidden form fields',
    prominentlyHighlightUnhidden: 'Prominently highlight unhidden fields',
    enableDisabledFields: 'Enable disabled form fields',
    removeInputFieldLengthLimits: 'Remove input field length limits',
    removeJavaScriptFormValidation: 'Remove JavaScript form validation',
    removeAllJavaScript: 'Remove all JavaScript',
    matchReplaceRules: 'Match and Replace Rules',
    matchReplaceRulesDesc: 'Configure match and replace rules for requests and responses',
    onlyApplyToInScope: 'Only apply to in-scope items',
    enabled: 'Enabled',
    item: 'Item',
    match: 'Match',
    replace: 'Replace',
    type: 'Type',
    comment: 'Comment',
    add: 'Add',
    edit: 'Edit',
    remove: 'Remove',
    pasteURL: 'Paste URL',
    load: 'Load',
    tlsPassThrough: 'TLS Pass Through',
    tlsPassThroughDesc: 'Configure TLS pass through rules',
    hostIPRange: 'Host/IP Range',
    noRules: 'No rules',
    autoAddTLSOnFailure: 'Auto-add TLS on failure',
    applyToOutOfScope: 'Apply to out-of-scope items',
    proxyHistoryLogging: 'Proxy History Logging',
    proxyHistoryLoggingDesc: 'Configure proxy history logging settings',
    stopLoggingOutOfScope: 'Stop logging out-of-scope items',
    askUser: 'Ask user',
    doNothing: 'Do Nothing',
    defaultInterceptionState: 'Default Interception State',
    defaultInterceptionStateDesc: 'Configure the default interception state for requests and responses',
    enableInterception: 'Enable Interception',
    disableInterception: 'Disable Interception',
    restoreInterceptionState: 'Restore Interception State',
    miscellaneousSettings: 'Miscellaneous Settings',
    miscellaneousSettingsDesc: 'Configure miscellaneous proxy settings',
    useHTTP1_1ToServer: 'Use HTTP/1.1 to Server',
    useHTTP1_1ToClient: 'Use HTTP/1.1 to Client',
    setConnectionClose: 'Set Connection: Close',
    setConnectionHeader: 'Set Connection Header',
    stripProxyHeaders: 'Strip Proxy Headers',
    removeUnsupportedEncodings: 'Remove Unsupported Encodings',
    stripWebSocketExtensions: 'Strip WebSocket Extensions',
    unpackCompressedRequests: 'Unpack Compressed Requests',
    unpackCompressedResponses: 'Unpack Compressed Responses',
    suppressBurpErrorMessages: 'Suppress Burp Error Messages',
    dontSendToProxyHistory: 'Don\'t Send to Proxy History',
    dontSendToProxyHistoryIfOutOfScope: 'Don\'t Send to Proxy History if Out-of-Scope',
    resetToDefaults: 'Reset to Defaults',
    saving: 'Saving...',
    // Upstream proxy
    upstreamProxyServers: 'Upstream Proxy Servers',
    upstreamProxyServersDesc: 'Configure upstream proxy servers to forward proxy requests. Create a rule with * as the destination host to send all traffic to a single proxy server.',
    destinationHost: 'Destination Host',
    destinationHostHelp: 'Use * to match all hosts',
    proxyHost: 'Proxy Host',
    proxyPort: 'Proxy Port',
    authType: 'Auth Type',
    authNone: 'None',
    authBasic: 'Basic',
    username: 'Username',
    password: 'Password',
    noUpstreamProxy: 'No upstream proxy configured',
    addUpstreamProxy: 'Add upstream proxy server',
    editUpstreamProxy: 'Edit upstream proxy server',
    // Match and Replace
    addMatchReplaceRule: 'Add match/replace rule',
    editMatchReplaceRule: 'Edit match/replace rule',
    matchPlaceholder: 'Enter regex pattern to match',
    replacePlaceholder: 'Enter replacement text',
    matchReplaceTypes: {
      requestHeader: 'Request header',
      requestBody: 'Request body',
      requestParamName: 'Request param name',
      requestParamValue: 'Request param value',
      requestFirstLine: 'Request first line',
      responseHeader: 'Response header',
      responseBody: 'Response body'
    },
    // TLS Pass Through
    addTlsPassThrough: 'Add TLS pass through rule',
    editTlsPassThrough: 'Edit TLS pass through rule',
    hostPlaceholder: 'e.g. *.example.com or 192.168.1.*'
  },
  // Proxifier Panel Component
  proxifier: {
    title: 'Proxifier',
    status: {
      running: 'Running',
      stopped: 'Stopped'
    },
    buttons: {
      start: 'Start',
      stop: 'Stop'
    },
    tabs: {
      proxies: 'Proxies',
      rules: 'Rules',
      system: 'System'
    }
  },
  // Proxifier Proxies Component
  proxifierProxies: {
    title: 'Proxy Servers',
    table: {
      name: 'Name',
      port: 'Port',
      type: 'Type',
      noProxies: 'No proxy servers'
    },
    emptyState: {
      noProxies: 'No proxy servers'
    },
    buttons: {
      add: 'Add...',
      edit: 'Edit',
      remove: 'Remove',
      proxyChains: 'Proxy Chains...'
    },
    description: 'Configure proxy chains to chain multiple proxy servers',
    dialog: {
      addTitle: 'Add Proxy Server',
      editTitle: 'Edit Proxy Server',
      host: {
        label: 'Host',
        placeholder: 'Enter host address'
      },
      port: {
        label: 'Port',
        placeholder: 'Enter port number'
      },
      type: {
        label: 'Type',
        http: 'HTTP',
        https: 'HTTPS',
        socks5: 'SOCKS5'
      },
      auth: 'Authentication',
      username: {
        label: 'Username',
        placeholder: 'Enter username (optional)'
      },
      password: {
        label: 'Password',
        placeholder: 'Enter password (optional)'
      },
      buttons: {
        cancel: 'Cancel',
        save: 'Save'
      }
    }
  },
  // Proxifier Panel Component
  proxifierPanel: {
    statusRunning: 'Running',
    statusStopped: 'Stopped',
    start: 'Start',
    stop: 'Stop',
    application: 'Application',
    target: 'Target',
    timeOrStatus: 'Time/Status',
    ruleProxy: 'Rule/Proxy',
    sent: 'Sent',
    received: 'Received',
    noConnections: 'No connections',
    startProxifierToShow: 'Start Proxifier to show connections',
    noLogs: 'No logs',
    transparentProxy: 'Transparent Proxy',
    transparentProxyStatus: 'Transparent Proxy Status',
    status: 'Status',
    running: 'Running',
    stopped: 'Stopped',
    enabled: 'Enabled',
    disabled: 'Disabled',
    proxyPort: 'Proxy Port',
    redirectPorts: 'Redirect Ports',
    startTransparentProxy: 'Start Transparent Proxy',
    stopTransparentProxy: 'Stop Transparent Proxy',
    transparentProxyDesc: 'transparently intercepts traffic from all applications',
    startTransparentProxyDesc: 'Start transparent proxy to automatically intercept traffic from all applications',
    stopTransparentProxyDesc: 'Stop transparent proxy to disable automatic traffic interception',
    pfFirewall: 'pf Firewall',
    rules: {
      title: 'Proxy Rules',
      noRules: 'No rules',
      table: {
        name: 'Name',
        applications: 'Applications',
        applicationsExample: 'Example: Chrome, Safari',
        targetHosts: 'Target Hosts',
        targetHostsExample: 'Example: *.example.com',
        targetPorts: 'Target Ports',
        targetPortsExample: 'Example: 80, 443',
        action: 'Action',
        direct: 'Direct',
        block: 'Block',
        viaProxy: 'Via Proxy',
        proxyFormat: '{type} {host}:{port}'
      }
    },
    buttons: {
      add: 'Add',
      clone: 'Clone',
      edit: 'Edit',
      remove: 'Remove',
      enabled: 'Enabled',
      cancel: 'Cancel',
      save: 'Save',
      close: 'Close'
    }
  },
  // Proxifier Rules Component
  rules: {
    title: 'Proxy Rules',
    table: {
      select: 'Select',
      name: 'Name',
      applications: 'Applications',
      targetHosts: 'Target Hosts',
      port: 'Port',
      action: 'Action',
      noRules: 'No rules'
    }
  },
  workflowStudio: {
    status: {
      saved: 'Saved',
      saving: 'Saving...',
      unsaved: 'Unsaved changes'
    },
    title: 'Workflow Studio',
    header: {
      namePlaceholder: 'Workflow name',
      editMetadataTooltip: 'Edit workflow metadata'
    },
    toolbar: {
      workflowList: 'Workflows',
      workflowListTooltip: 'Open workflow list',
      load: 'Load',
      loadTooltip: 'Load workflow',
      templates: 'Templates',
      templateMarketTooltip: 'Template Market',
      save: 'Save',
      saveTooltip: 'Save workflow',
      exportImportTooltip: 'Export/Import',
      refreshCatalog: 'Refresh catalog',
      refreshCatalogTooltip: 'Refresh node catalog',
      resetCanvas: 'Reset canvas',
      resetCanvasTooltip: 'Reset canvas',
      run: 'Run',
      runTooltip: 'Run workflow',
      stop: 'Stop',
      stopTooltip: 'Stop workflow',
      schedule: 'Schedule',
      startScheduleTooltip: 'Start schedule',
      startScheduleDisabledTooltip: 'Save workflow and add a schedule trigger node first',
      stopScheduleTooltip: 'Stop schedule',
      logs: 'Logs',
      toggleLogsTooltip: 'Toggle logs panel',
      history: 'History',
      executionHistoryTooltip: 'Execution history'
    },
    export: {
      exportJson: 'Export as JSON',
      importJson: 'Import from JSON',
      exportImage: 'Export as image',
      exportedBy: 'Exported by Sentinel AI Workflow Studio'
    },
    sidebar: {
      nodeLibrary: 'Node Library',
      expandSidebar: 'Expand sidebar',
      collapseSidebar: 'Collapse sidebar',
      searchPlaceholder: 'Search nodes...',
      clearSearchTooltip: 'Clear search',
      searchInCanvasTooltip: 'Search in canvas',
      favoritesOnly: 'Favorites only',
      noMatchingNodes: 'No matching nodes',
      favorite: 'Favorite',
      unfavorite: 'Unfavorite'
    },
    logs: {
      title: 'Execution Logs',
      clear: 'Clear',
      empty: 'No logs',
      expandDetails: 'Expand details',
      collapseDetails: 'Collapse details',
      executionId: 'Execution ID: {id}',
      newWorkflowCreated: 'New workflow created',
      validationFailed: 'Workflow validation failed: {message}',
      validationError: 'Validation error: {error}',
      workflowExecutionStarted: 'Workflow execution started: {name}',
      workflowStarted: 'Workflow started',
      startFailed: 'Start failed: {error}',
      stoppingWorkflow: 'Stopping workflow...',
      workflowStopped: 'Workflow stopped',
      stopFailed: 'Stop failed: {error}',
      scheduleStarted: 'Schedule started: {desc}',
      scheduleStartFailed: 'Failed to start schedule: {error}',
      scheduleStopped: 'Schedule stopped',
      scheduleStopFailed: 'Failed to stop schedule: {error}',
      workflowSaved: 'Workflow saved: {name}',
      workflowSavedAsTool: 'Workflow saved: {name} (set as tool)',
      saveFailed: 'Save failed: {error}',
      workflowLoaded: 'Workflow loaded: {name}',
      loadFailed: 'Load failed: {error}',
      workflowExported: 'Workflow exported: {filename}',
      exportFailed: 'Export failed: {error}',
      workflowImported: 'Workflow imported: {name}',
      importFailed: 'Import failed: {message}',
      imageExportTodo: 'Image export feature is not implemented yet',
      workflowExecutionStartedExternal: 'Workflow execution started (external trigger)',
      nodeStarted: 'Node execution started',
      nodeCompleted: 'Node execution completed',
      workflowCompleted: 'Workflow execution completed',
      workflowExecutionStopped: 'Workflow execution stopped',
      foundMatchingNodes: 'Found {count} matching nodes',
      noMatchingNodes: 'No matching nodes found',
      templateSaved: 'Template saved: {name}',
      templateSaveFailed: 'Failed to save template: {error}'
    },
    loadDialog: {
      title: 'Load Workflow',
      empty: 'No saved workflows',
      version: 'Version: {version}',
      updated: 'Updated: {date}',
      deleteTooltip: 'Delete',
      close: 'Close'
    },
    workflowListPanel: {
      title: 'Workflow Management',
      myWorkflows: 'My Workflows',
      templates: 'Templates',
      searchPlaceholder: 'Search workflows or templates...',
      emptyWorkflows: 'No saved workflows',
      emptyTemplates: 'No templates',
      aiTool: 'AI Tool',
      duplicate: 'Duplicate',
      delete: 'Delete',
      templateBadge: 'Template',
      nodeCount: '{count} nodes',
      useTemplate: 'Use template',
      deleteTemplate: 'Delete template',
      newWorkflow: 'New Workflow',
      saveAsTemplate: 'Save as Template'
    },
    templateMarket: {
      title: 'Workflow Template Market',
      recommended: 'Recommended',
      myTemplates: 'My Templates',
      empty: 'No templates',
      templateBadge: 'Template',
      nodeCount: '{count} nodes',
      useTemplate: 'Use template',
      saveAsTemplate: 'Save as template',
      saveCurrentAsTemplate: 'Save current as template',
      close: 'Close'
    },
    newWorkflowConfirm: {
      title: 'New Workflow',
      message: 'The current workflow is not saved. Save it before creating a new one?',
      saveAndNew: 'Save and create new',
      discardAndNew: 'Discard and create new',
      cancel: 'Cancel',
      close: 'Close'
    },
    metaDialog: {
      title: 'Workflow Metadata',
      name: 'Workflow name',
      namePlaceholder: 'Enter workflow name',
      description: 'Description',
      descriptionPlaceholder: 'Describe the purpose and functionality of this workflow',
      tags: 'Tags',
      tagsPlaceholder: 'Separate multiple tags with commas, e.g. automation,data processing',
      version: 'Version',
      asAiTool: 'Expose as AI tool',
      asAiToolHelp: 'When enabled, this workflow can be used as a tool by the AI assistant',
      stats: {
        nodes: 'Nodes',
        edges: 'Edges'
      },
      confirm: 'Confirm',
      cancel: 'Cancel'
    },
    paramsEditor: {
      title: 'Parameter Editor',
      noParams: 'This node has no configurable parameters',
      selectNotificationRule: '-- Select notification rule --',
      noNotificationRules: 'No notification rules available,',
      goToConfigure: 'Go to configure',
      useDefaultConfig: '-- Use default configuration --',
      noAiProviders: 'No AI providers available,',
      selectModel: 'Select model',
      selectProviderFirst: 'Select provider first',
      noTools: 'No available tools',
      selectedToolsCount: '{count} tools selected',
      enterField: 'Enter {key}',
      onePerLine: 'One value per line',
      pleaseSelect: '-- Please select --',
      booleanYes: 'Yes',
      booleanNo: 'No',
      arrayPlaceholder: 'One value per line, for example:\nhttps://example1.com/\nhttps://example2.com/',
      defaultValue: 'Default: {value}',
      save: 'Save',
      cancel: 'Cancel'
    },
    executionHistory: {
      title: 'Execution History',
      clear: 'Clear',
      clearTooltip: 'Clear history',
      emptyTitle: 'No execution records',
      emptyDescription: 'Run the workflow to see history here',
      searchPlaceholder: 'Search workflow name...',
      status: {
        completed: 'Completed',
        failed: 'Failed',
        running: 'Running',
        pending: 'Pending',
        cancelled: 'Cancelled'
      },
      deleteRecordTooltip: 'Delete this record',
      durationMs: 'Duration: {ms}ms',
      detailsTitle: 'Execution Details',
      copyResultsTooltip: 'Copy results',
      table: {
        name: 'Execution Name',
        startTime: 'Start Time',
        duration: 'Duration',
        status: 'Status',
        actions: 'Actions',
        viewDetail: 'View Details',
        delete: 'Delete'
      },
      pagination: {
        total: 'Total {total}'
      },
      detailDialog: {
        title: 'Execution Details',
        workflowName: 'Workflow',
        status: 'Status',
        startTime: 'Start Time',
        duration: 'Duration',
        error: 'Error',
        steps: 'Steps',
        noSteps: 'No step records',
        noResult: 'No result',
        copy: 'Copy',
        close: 'Close',
        fullscreen: 'Fullscreen',
        exitFullscreen: 'Exit Fullscreen'
      }
    },
    resultPanel: {
      title: 'Step Execution Result',
      copyTooltip: 'Copy result',
      nodeId: 'Node ID',
      nodeName: 'Node name',
      unknown: 'Unknown',
      executionResult: 'Execution result',
      editParams: 'Edit parameters',
      close: 'Close',
      noResult: 'No result'
    },
    groups: {
      trigger: 'Trigger',
      control: 'Control Flow',
      ai: 'AI',
      data: 'Data',
      output: 'Output/Notification',
      tool: 'Built-in Tools',
      mcp: 'MCP Tools',
      plugin: 'Agent Plugins'
    },
    schedule: {
      everySeconds: 'Every {seconds} seconds',
      dailyAt: 'Daily at {time}',
      weeklyAt: 'Weekly on {weekdays} at {time}'
    },
    confirm: {
      deleteWorkflow: 'Are you sure you want to delete this workflow?',
      deleteTemplate: 'Are you sure you want to delete this template?'
    },
    toasts: {
      enterWorkflowName: 'Please enter a workflow name first',
      newWorkflowCreated: 'New workflow created',
      copiedToClipboard: 'Result copied to clipboard',
      copyFailed: 'Copy failed: {message}',
      validationFailed: 'Validation failed: {message}',
      validationError: 'Validation error: {error}',
      executionStarted: 'Execution started: {id}',
      startFailed: 'Start failed: {error}',
      noRunningWorkflow: 'No running workflow',
      workflowStopped: 'Workflow stopped',
      stopFailed: 'Stop failed: {error}',
      scheduleMissingTrigger: 'Add and configure a schedule trigger node first',
      scheduleStarted: 'Schedule started: {desc}',
      scheduleStartFailed: 'Failed to start schedule: {error}',
      scheduleStopped: 'Schedule stopped',
      scheduleStopFailed: 'Failed to stop schedule: {error}',
      workflowSaved: 'Workflow saved',
      saveFailed: 'Save failed: {error}',
      loadFailed: 'Load failed: {error}',
      workflowDeleted: 'Workflow deleted',
      deleteFailed: 'Delete failed: {error}',
      workflowExported: 'Workflow exported',
      exportFailed: 'Export failed: {error}',
      workflowImported: 'Workflow imported',
      importFailed: 'Import failed: {message}',
      imageExportRequiresHtml2Canvas: 'Image export requires the html2canvas library',
      templateSaved: 'Template saved',
      templateSaveFailed: 'Failed to save template: {error}',
      workflowDuplicated: 'Workflow duplicated',
      duplicateFailed: 'Duplicate failed: {error}',
      templateDeleted: 'Template deleted',
      deleteTemplateFailed: 'Failed to delete template: {error}',
      templateApplied: 'Template applied',
      applyTemplateFailed: 'Failed to apply template: {error}'
    },
    flowchart: {
      toolbar: {
        title: 'Execution Flowchart',
        newWorkflow: 'New',
        newWorkflowTooltip: 'Create new workflow',
        aiGenerate: 'AI Generate',
        aiGenerateTooltip: 'Generate workflow from natural language description',
        fitToViewTooltip: 'Fit to view (Ctrl+0)',
        resetViewTooltip: 'Reset view (Ctrl+1)',
        minimapTooltip: 'Minimap',
        arrangeNodes: 'Arrange nodes',
        arrangeNodesTooltip: 'Automatically arrange node layout',
        undoTooltip: 'Undo (Ctrl+Z)',
        redoTooltip: 'Redo (Ctrl+Y)',
        deleteConnection: 'Delete connection',
        deleteConnectionTooltip: 'Click a connection line to delete',
        exitFullscreen: 'Exit fullscreen'
      },
      emptyState: {
        title: 'Canvas is empty',
        description: 'Drag nodes from the node library on the left to start building a workflow',
        tip: 'Tip: drag on blank area to pan, scroll to zoom, Ctrl+drag to select'
      },
      canvasHints: {
        space: 'Drag blank: Pan',
        scroll: 'Scroll: Zoom',
        drag: 'Ctrl+drag: Select',
        selectAll: 'Select all',
        selected: '{count} nodes selected'
      },
      ports: {
        input: 'Input',
        output: 'Output'
      },
      breakpoints: {
        title: 'Breakpoints'
      },
      status: {
        pending: 'Pending',
        planning: 'Planning',
        running: 'Running',
        completed: 'Completed',
        failed: 'Failed',
        paused: 'Paused',
        cancelled: 'Cancelled'
      },
      contextMenu: {
        addBreakpoint: 'Add breakpoint',
        removeBreakpoint: 'Remove breakpoint',
        duplicateNode: 'Duplicate node',
        deleteNode: 'Delete node',
        duplicateNodeName: '{name} (Copy)'
      },
      aiGenerate: {
        title: 'AI Generated Workflow',
        help: 'Describe the workflow you want in natural language, for example: first run subdomain scan, then port scan, then use AI to analyze results and generate a report.',
        placeholder: 'Enter workflow description...',
        cancel: 'Cancel',
        generateAndLoad: 'Generate and load',
        missingNodesError: 'Generated result is missing nodes'
      }
    },
    defaults: {
      unnamedWorkflow: 'Unnamed workflow',
      importedWorkflow: 'Imported workflow',
      duplicateWorkflowName: '{name} (Copy)'
    },
    errors: {
      invalidWorkflowFile: 'Invalid workflow file format',
      jsonFormatError: 'JSON format error: {message}'
    }
  }
}
