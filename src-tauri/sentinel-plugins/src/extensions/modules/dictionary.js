export function getDictionary(idOrName) {
    return __sentinel_get_dictionary(idOrName);
}

export function getDefaultId(dictType) {
    return __sentinel_get_default_dictionary_id(dictType);
}

export function getWords(idOrName, limit) {
    return __sentinel_get_dictionary_words(idOrName, limit);
}

export function getEntries(idOrName, limit) {
    return __sentinel_get_dictionary_entries(idOrName, limit);
}

export function list(filter) {
    var dictType = null;
    var category = null;
    if (filter) {
        if (filter.dictType) dictType = filter.dictType;
        if (filter.category) category = filter.category;
    }
    return __sentinel_list_dictionaries(dictType, category);
}
