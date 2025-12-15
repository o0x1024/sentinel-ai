export default {
    title: 'RAG Knowledge Base Management',

    // Buttons
    createCollection: 'Create Collection',
    refresh: 'Refresh',
    addDocument: 'Add Document',
    queryDocument: 'Query Document',
    close: 'Close',
    cancel: 'Cancel',
    save: 'Save',
    create: 'Create',
    creating: 'Creating...',
    saving: 'Saving...',
    processing: 'Processing...',
    querying: 'Querying...',

    // Statistics Cards
    stats: {
        totalCollections: 'Total Collections',
        totalDocuments: 'Total Documents',
        totalChunks: 'Total Chunks',
        totalQueries: 'Total Queries',
    },

    // Collection List
    collections: {
        title: 'Knowledge Base Collections',
        description: 'Activated collections will be jointly retrieved in AI assistant RAG mode.',
        searchPlaceholder: 'Search collections...',
        allStatus: 'All Status',
        active: 'Active',
        inactive: 'Inactive',
        activated: 'Activated',
        notActivated: 'Not Activated',
    },

    // Table Columns
    table: {
        name: 'Name',
        description: 'Description',
        embeddingModel: 'Embedding Model',
        documentCount: 'Documents',
        createdAt: 'Created At',
        activate: 'Activate',
        actions: 'Actions',
        fileName: 'File Name',
        size: 'Size',
        operations: 'Operations',
        chunks: 'Chunks',
    },

    // Actions
    actions: {
        view: 'View Details',
        edit: 'Edit Collection',
        add: 'Add Document',
        query: 'Query',
        delete: 'Delete',
        preview: 'Preview',
    },

    // Create/Edit Collection
    collection: {
        createTitle: 'Create New Collection',
        editTitle: 'Edit Collection',
        nameLabel: 'Collection Name',
        namePlaceholder: 'Enter collection name',
        descriptionLabel: 'Description',
        descriptionPlaceholder: 'Enter collection description',
        noDescription: 'No description',
    },

    // Document Ingestion
    ingest: {
        title: 'Add Document to {name}',
        modeFile: 'Select File',
        modeManual: 'Manual Input',
        selectFile: 'Select File',
        selectSingleFile: 'Select Single File',
        selectFolder: 'Select Folder',
        supportedFormats: 'Supported formats: TXT, MD, PDF, DOCX',
        filesSelected: '{count} files selected',
        fileSelected: 'File selected: {name}',
        folder: 'Folder: {path}',
        fileList: 'File list:',
        titleLabel: 'Document Title',
        titleRequired: 'Document Title *',
        titlePlaceholder: 'Enter document title...',
        contentLabel: 'Document Content',
        contentRequired: 'Document Content *',
        contentPlaceholder: 'Enter or paste document content...',
        characterCount: 'Characters: {count}',
        addMultiple: 'Add {count} documents',
        processingProgress: 'Processing... {current}/{total}',
    },

    // Collection Details
    details: {
        title: 'Collection Details: {name}',
        documentCount: 'Document Count',
        chunkCount: 'Chunk Count',
        embeddingModel: 'Embedding Model',
        description: 'Description',
        documentList: 'Document List',
        searchPlaceholder: 'Search by filename...',
        perPage: '{size}/page',
        loading: 'Loading documents...',
        noDocuments: 'No documents',
        prevPage: 'Previous',
        nextPage: 'Next',
        pageInfo: 'Page {current} / {total} (Total {count})',
    },

    // Document Preview
    document: {
        previewTitle: 'Document Preview: {name}',
        chunks: 'Chunks: {count}',
        loadingContent: 'Loading content...',
    },

    // Query
    query: {
        title: 'Query {name}',
        contentLabel: 'Query Content',
        contentPlaceholder: 'Enter your query...',
        topKLabel: 'Number of Results',
        useEmbeddingLabel: 'Use Embedding Retrieval',
        rerankingLabel: 'Enable Reranking',
        execute: 'Execute Query',
        resultsTitle: 'Query Results',
        similarity: 'Similarity: {score}%',
        rank: 'Rank: {rank}',
    },

    // Messages
    messages: {
        activateSuccess: 'Collection activated, AI assistant will retrieve jointly',
        deactivateSuccess: 'Collection deactivated',
        updateActiveFailed: 'Failed to update activation status',
        createSuccess: 'Collection created successfully',
        createFailed: 'Failed to create collection: {error}',
        updateSuccess: 'Collection updated successfully',
        updateFailed: 'Failed to update collection: {error}',
        deleteSuccess: 'Collection deleted successfully',
        deleteFailed: 'Failed to delete collection: {error}',
        loadFailed: 'Failed to load collection list',
        loadDetailsFailed: 'Failed to load collection details: {error}',
        loadDocumentsFailed: 'Failed to load document list',
        loadChunksFailed: 'Failed to load document content',
        deleteDocumentSuccess: 'Document deleted successfully',
        deleteDocumentFailed: 'Failed to delete document: {error}',
        selectCollection: 'Please select a collection',
        selectFile: 'Please select a file',
        fillRequired: 'Please fill in document title and content',
        ingestSuccess: 'Document added successfully! Processed {chunks} chunks',
        ingestPartialSuccess: 'Partial success! Success: {success}, Failed: {failed}, Processed {chunks} chunks',
        ingestAllFailed: 'All documents failed to add',
        ingestFailed: 'Document ingestion failed: {error}',
        querySuccess: 'Found {count} relevant results',
        queryFailed: 'Query failed: {error}',
        queryRequired: 'Please enter query content',
        nameRequired: 'Please fill in collection name',
        fileSelectFailed: 'File selection failed: {error}',
        folderSelectFailed: 'Folder selection failed: {error}',
        folderFilesFailed: 'Failed to get folder files: {error}',
        noSupportedFiles: 'No supported document files found in folder',
        foundFiles: 'Found {count} document files',
    },
}
