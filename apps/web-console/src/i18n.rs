use crate::state::Language;
use wasm_bindgen::prelude::wasm_bindgen;

// Translation table is compiled in from assets/i18n/zh-CN.json.
// JS receives it once via init_translations() before any DOM language switch.
const ZH_CN_JSON: &str = include_str!("../../../assets/i18n/zh-CN.json");

#[wasm_bindgen(inline_js = r#"
let translations = new Map();
const translatedValues = new Set();

export function initTranslations(jsonStr) {
  const obj = JSON.parse(jsonStr);
  translations = new Map(Object.entries(obj));
  translations.forEach(v => translatedValues.add(v));
}

const originalText = new WeakMap();
const originalAttrs = new WeakMap();
let currentLanguage = "en";
let observer = null;
let applying = false;

function withOriginalSpacing(source, translated) {
  const prefix = source.match(/^\s*/)?.[0] ?? "";
  const suffix = source.match(/\s*$/)?.[0] ?? "";
  return `${prefix}${translated}${suffix}`;
}

function sourceForTextNode(node) {
  const value = node.nodeValue ?? "";
  const trimmed = value.trim();
  let source = originalText.get(node);
  if (!source || (trimmed !== source.trim() && !translatedValues.has(trimmed))) {
    source = value;
    originalText.set(node, source);
  }
  return source;
}

function translateTextNode(node, language) {
  const source = sourceForTextNode(node);
  const key = source.trim();
  if (!key) return;
  if (language === "zh-CN") {
    const translated = translations.get(key);
    if (translated) {
      const nextValue = withOriginalSpacing(source, translated);
      if (node.nodeValue !== nextValue) node.nodeValue = nextValue;
    }
  } else {
    if (node.nodeValue !== source) node.nodeValue = source;
  }
}

function rememberAttr(element, attr) {
  const value = element.getAttribute(attr);
  if (!value) return null;
  let attrs = originalAttrs.get(element);
  if (!attrs) {
    attrs = {};
    originalAttrs.set(element, attrs);
  }
  const trimmed = value.trim();
  if (!attrs[attr] || (trimmed !== attrs[attr].trim() && !translatedValues.has(trimmed))) {
    attrs[attr] = value;
  }
  return attrs[attr];
}

function translateAttr(element, attr, language) {
  const source = rememberAttr(element, attr);
  if (!source) return;
  if (language === "zh-CN") {
    const translated = translations.get(source.trim());
    if (translated && element.getAttribute(attr) !== translated) element.setAttribute(attr, translated);
  } else {
    if (element.getAttribute(attr) !== source) element.setAttribute(attr, source);
  }
}

function shouldSkipText(parent) {
  if (!parent) return true;
  const tagName = parent.nodeName;
  return tagName === "SCRIPT" || tagName === "STYLE" || tagName === "PRE" || tagName === "CODE";
}

function translateRoot(root, language) {
  applying = true;
  try {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    for (const node of nodes) {
      if (!shouldSkipText(node.parentElement)) translateTextNode(node, language);
    }
    for (const element of root.querySelectorAll("[placeholder],[aria-label],[title]")) {
      translateAttr(element, "placeholder", language);
      translateAttr(element, "aria-label", language);
      translateAttr(element, "title", language);
    }
  } finally {
    applying = false;
  }
}

export function applyDocumentLanguage(language) {
  currentLanguage = language;
  document.documentElement.lang = language;
  const root = document.querySelector(".app");
  if (!root) return;
  translateRoot(root, language);
  if (observer) return;
  observer = new MutationObserver(() => {
    if (applying || currentLanguage !== "zh-CN") return;
    const appRoot = document.querySelector(".app");
    if (appRoot) translateRoot(appRoot, currentLanguage);
  });
  observer.observe(root, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ["placeholder", "aria-label", "title"]
  });
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = initTranslations)]
    fn init_translations(json: &str);

    #[wasm_bindgen(js_name = applyDocumentLanguage)]
    pub(crate) fn apply_document_language(language: &str);
}

/// Call once at startup to push the compiled-in JSON into the JS translation Map.
pub(crate) fn setup_translations() {
    init_translations(ZH_CN_JSON);
}

pub(crate) fn tr(language: Language, en: &'static str, zh: &'static str) -> &'static str {
    match language {
        Language::En => en,
        Language::Zh => zh,
    }
}
