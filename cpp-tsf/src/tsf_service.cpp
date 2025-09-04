#include "../include/tsf_service.h"
#include <iostream>
#include <string>
#include <locale>
#include <codecvt>
#include <vector>

// GUID定义
const GUID CLSID_YiTextService = 
{ 0x12345678, 0x1234, 0x5678, { 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0 } };

const GUID GUID_YiProfile = 
{ 0x87654321, 0x4321, 0x8765, { 0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f } };

// 全局变量
YiTextService* g_pTextService = nullptr;
HINSTANCE g_hInst = nullptr;

// DLL入口点
BOOL APIENTRY DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved) {
    switch (fdwReason) {
    case DLL_PROCESS_ATTACH:
        g_hInst = hinstDLL;
        DisableThreadLibraryCalls(hinstDLL);
        break;
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}

// YiTextService实现
YiTextService::YiTextService() {
    m_cRef = 1;
    m_pThreadMgr = nullptr;
    m_tfClientId = TF_CLIENTID_NULL;
    m_pContext = nullptr;
    m_dwThreadMgrEventSinkCookie = TF_INVALID_COOKIE;
    m_dwTextEditSinkCookie = TF_INVALID_COOKIE;
    m_dwKeyEventSinkCookie = TF_INVALID_COOKIE;
}

YiTextService::~YiTextService() {
    Deactivate();
}

// IUnknown接口实现
STDMETHODIMP YiTextService::QueryInterface(REFIID riid, void **ppvObj) {
    if (ppvObj == nullptr) return E_INVALIDARG;
    
    *ppvObj = nullptr;
    
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfTextInputProcessor)) {
        *ppvObj = (ITfTextInputProcessor*)this;
    }
    else if (IsEqualIID(riid, IID_ITfThreadMgrEventSink)) {
        *ppvObj = (ITfThreadMgrEventSink*)this;
    }
    else if (IsEqualIID(riid, IID_ITfTextEditSink)) {
        *ppvObj = (ITfTextEditSink*)this;
    }
    else if (IsEqualIID(riid, IID_ITfKeyEventSink)) {
        *ppvObj = (ITfKeyEventSink*)this;
    }
    
    if (*ppvObj) {
        AddRef();
        return S_OK;
    }
    
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) YiTextService::AddRef() {
    return ++m_cRef;
}

STDMETHODIMP_(ULONG) YiTextService::Release() {
    LONG cr = --m_cRef;
    if (cr == 0) {
        delete this;
    }
    return cr;
}

// ITfTextInputProcessor接口实现
STDMETHODIMP YiTextService::Activate(ITfThreadMgr *pThreadMgr, TfClientId tfClientId) {
    m_pThreadMgr = pThreadMgr;
    m_tfClientId = tfClientId;
    
    if (m_pThreadMgr) {
        m_pThreadMgr->AddRef();
    }
    
    _InitThreadMgrSink();
    _InitKeyEventSink();
    
    return S_OK;
}

STDMETHODIMP YiTextService::Deactivate() {
    _UninitKeyEventSink();
    _UninitTextEditSink();
    _UninitThreadMgrSink();
    
    if (m_pContext) {
        m_pContext->Release();
        m_pContext = nullptr;
    }
    
    if (m_pThreadMgr) {
        m_pThreadMgr->Release();
        m_pThreadMgr = nullptr;
    }
    
    m_tfClientId = TF_CLIENTID_NULL;
    
    return S_OK;
}

// ITfThreadMgrEventSink接口实现
STDMETHODIMP YiTextService::OnInitDocumentMgr(ITfDocumentMgr *pDocMgr) {
    return S_OK;
}

STDMETHODIMP YiTextService::OnUninitDocumentMgr(ITfDocumentMgr *pDocMgr) {
    return S_OK;
}

STDMETHODIMP YiTextService::OnSetFocus(ITfDocumentMgr *pDocMgrFocus, ITfDocumentMgr *pDocMgrPrevFocus) {
    _UninitTextEditSink();
    
    if (pDocMgrFocus) {
        _InitTextEditSink(pDocMgrFocus);
    }
    
    return S_OK;
}

STDMETHODIMP YiTextService::OnPushContext(ITfContext *pContext) {
    return S_OK;
}

STDMETHODIMP YiTextService::OnPopContext(ITfContext *pContext) {
    return S_OK;
}

// ITfTextEditSink接口实现
STDMETHODIMP YiTextService::OnEndEdit(ITfContext *pContext, TfEditCookie ecReadOnly, ITfEditRecord *pEditRecord) {
    return S_OK;
}

// ITfKeyEventSink接口实现
STDMETHODIMP YiTextService::OnSetFocus(BOOL fForeground) {
    return S_OK;
}

STDMETHODIMP YiTextService::OnTestKeyDown(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten) {
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP YiTextService::OnKeyDown(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten) {
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP YiTextService::OnTestKeyUp(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten) {
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP YiTextService::OnKeyUp(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten) {
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP YiTextService::OnPreservedKey(ITfContext *pContext, REFGUID rguid, BOOL *pfEaten) {
    *pfEaten = FALSE;
    return S_OK;
}

// 文本插入实现
HRESULT YiTextService::InsertTextViaSendInput(const WCHAR *pszText) {
    if (!pszText) {
        return E_FAIL;
    }
    
    size_t len = wcslen(pszText);
    if (len == 0) {
        return S_OK;
    }
    
    printf("InsertTextViaSendInput: 使用SendInput插入文本: %ls\n", pszText);
    
    // 创建INPUT结构数组
    std::vector<INPUT> inputs;
    inputs.reserve(len * 2); // 每个字符需要按下和释放
    
    for (size_t i = 0; i < len; i++) {
        INPUT input = {0};
        input.type = INPUT_KEYBOARD;
        input.ki.wVk = 0;
        input.ki.wScan = pszText[i];
        input.ki.dwFlags = KEYEVENTF_UNICODE;
        input.ki.time = 0;
        input.ki.dwExtraInfo = 0;
        
        inputs.push_back(input);
        
        // 添加按键释放事件
        input.ki.dwFlags |= KEYEVENTF_KEYUP;
        inputs.push_back(input);
    }
    
    // 发送输入事件
    UINT sent = SendInput((UINT)inputs.size(), inputs.data(), sizeof(INPUT));
    
    printf("InsertTextViaSendInput: 发送了 %u/%zu 个输入事件\n", sent, inputs.size());
    
    return (sent == inputs.size()) ? S_OK : E_FAIL;
}

// 修改InsertText方法，添加备选方案
HRESULT YiTextService::InsertText(const WCHAR *pszText) {
    if (!pszText) {
        return E_FAIL;
    }
    
    // 首先尝试TSF方法
    if (!m_pContext) {
        HRESULT hr = _GetFocusContext();
        if (FAILED(hr)) {
            printf("InsertText: 获取焦点上下文失败，使用SendInput\n");
            return InsertTextViaSendInput(pszText);
        }
    }
    
    if (!m_pContext) {
        printf("InsertText: 上下文仍为空，使用SendInput\n");
        return InsertTextViaSendInput(pszText);
    }
    
    printf("InsertText: 创建EditSession\n");
    
    YiEditSession *pEditSession = new YiEditSession(this, pszText);
    if (!pEditSession) {
        return E_OUTOFMEMORY;
    }
    
    HRESULT hr = S_OK;
    // 请求同步编辑会话，确保立即执行
    HRESULT hrSession = m_pContext->RequestEditSession(m_tfClientId, pEditSession, TF_ES_READWRITE | TF_ES_SYNC, &hr);
    
    printf("InsertText: RequestEditSession返回: 0x%08X, 编辑结果: 0x%08X\n", hrSession, hr);
    
    pEditSession->Release();
    
    if (FAILED(hrSession)) {
        printf("InsertText: EditSession请求失败，使用SendInput\n");
        return InsertTextViaSendInput(pszText);
    }
    
    if (FAILED(hr)) {
        printf("InsertText: 编辑操作失败，使用SendInput\n");
        return InsertTextViaSendInput(pszText);
    }
    
    return hr;
}

// 私有方法实现
HRESULT YiTextService::_InitThreadMgrSink() {
    if (!m_pThreadMgr) return E_FAIL;
    
    ITfSource *pSource = nullptr;
    HRESULT hr = m_pThreadMgr->QueryInterface(IID_ITfSource, (void**)&pSource);
    if (SUCCEEDED(hr)) {
        hr = pSource->AdviseSink(IID_ITfThreadMgrEventSink, (ITfThreadMgrEventSink*)this, &m_dwThreadMgrEventSinkCookie);
        pSource->Release();
    }
    
    return hr;
}

HRESULT YiTextService::_UninitThreadMgrSink() {
    if (!m_pThreadMgr || m_dwThreadMgrEventSinkCookie == TF_INVALID_COOKIE) {
        return S_OK;
    }
    
    ITfSource *pSource = nullptr;
    HRESULT hr = m_pThreadMgr->QueryInterface(IID_ITfSource, (void**)&pSource);
    if (SUCCEEDED(hr)) {
        hr = pSource->UnadviseSink(m_dwThreadMgrEventSinkCookie);
        pSource->Release();
    }
    
    m_dwThreadMgrEventSinkCookie = TF_INVALID_COOKIE;
    return hr;
}

HRESULT YiTextService::_InitTextEditSink(ITfDocumentMgr *pDocMgr) {
    if (!pDocMgr) return E_FAIL;
    
    HRESULT hr = pDocMgr->GetTop(&m_pContext);
    if (FAILED(hr) || !m_pContext) {
        return hr;
    }
    
    ITfSource *pSource = nullptr;
    hr = m_pContext->QueryInterface(IID_ITfSource, (void**)&pSource);
    if (SUCCEEDED(hr)) {
        hr = pSource->AdviseSink(IID_ITfTextEditSink, (ITfTextEditSink*)this, &m_dwTextEditSinkCookie);
        pSource->Release();
    }
    
    return hr;
}

HRESULT YiTextService::_UninitTextEditSink() {
    if (!m_pContext || m_dwTextEditSinkCookie == TF_INVALID_COOKIE) {
        return S_OK;
    }
    
    ITfSource *pSource = nullptr;
    HRESULT hr = m_pContext->QueryInterface(IID_ITfSource, (void**)&pSource);
    if (SUCCEEDED(hr)) {
        hr = pSource->UnadviseSink(m_dwTextEditSinkCookie);
        pSource->Release();
    }
    
    m_dwTextEditSinkCookie = TF_INVALID_COOKIE;
    return hr;
}

HRESULT YiTextService::_InitKeyEventSink() {
    if (!m_pThreadMgr) return E_FAIL;
    
    ITfKeystrokeMgr *pKeystrokeMgr = nullptr;
    HRESULT hr = m_pThreadMgr->QueryInterface(IID_ITfKeystrokeMgr, (void**)&pKeystrokeMgr);
    if (SUCCEEDED(hr)) {
        hr = pKeystrokeMgr->AdviseKeyEventSink(m_tfClientId, (ITfKeyEventSink*)this, TRUE);
        pKeystrokeMgr->Release();
    }
    
    return hr;
}

HRESULT YiTextService::_UninitKeyEventSink() {
    if (!m_pThreadMgr) return S_OK;
    
    ITfKeystrokeMgr *pKeystrokeMgr = nullptr;
    HRESULT hr = m_pThreadMgr->QueryInterface(IID_ITfKeystrokeMgr, (void**)&pKeystrokeMgr);
    if (SUCCEEDED(hr)) {
        hr = pKeystrokeMgr->UnadviseKeyEventSink(m_tfClientId);
        pKeystrokeMgr->Release();
    }
    
    return hr;
}

HRESULT YiTextService::_GetFocusContext() {
    if (!m_pThreadMgr) {
        return E_FAIL;
    }
    
    // 尝试获取当前线程的焦点文档管理器
    ITfDocumentMgr *pDocMgr = nullptr;
    HRESULT hr = m_pThreadMgr->GetFocus(&pDocMgr);
    
    if (FAILED(hr) || !pDocMgr) {
        // 如果无法获取焦点，尝试枚举所有文档管理器
        IEnumTfDocumentMgrs *pEnumDocMgrs = nullptr;
        hr = m_pThreadMgr->EnumDocumentMgrs(&pEnumDocMgrs);
        if (SUCCEEDED(hr)) {
            ULONG fetched = 0;
            hr = pEnumDocMgrs->Next(1, &pDocMgr, &fetched);
            pEnumDocMgrs->Release();
            
            if (FAILED(hr) || fetched == 0) {
                return E_FAIL;
            }
        } else {
            return E_FAIL;
        }
    }
    
    if (pDocMgr) {
        // 清理旧的上下文
        _UninitTextEditSink();
        
        // 初始化新的上下文
        hr = _InitTextEditSink(pDocMgr);
        pDocMgr->Release();
    }
    
    return hr;
}

// C接口实现
extern "C" {
    int tsf_initialize() {
        if (g_pTextService) {
            return 0; // 已经初始化
        }
        
        HRESULT hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
        if (FAILED(hr)) {
            printf("COM初始化失败: 0x%08X\n", hr);
            return -1;
        }
        
        g_pTextService = new YiTextService();
        if (!g_pTextService) {
            CoUninitialize();
            return -2;
        }
        
        ITfThreadMgr *pThreadMgr = nullptr;
        hr = CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_ITfThreadMgr, (void**)&pThreadMgr);
        if (FAILED(hr)) {
            printf("ThreadMgr创建失败: 0x%08X\n", hr);
            delete g_pTextService;
            g_pTextService = nullptr;
            CoUninitialize();
            return -3;
        }
        
        TfClientId clientId;
        hr = pThreadMgr->Activate(&clientId);
        if (FAILED(hr)) {
            printf("ThreadMgr激活失败: 0x%08X\n", hr);
            pThreadMgr->Release();
            delete g_pTextService;
            g_pTextService = nullptr;
            CoUninitialize();
            return -3;
        }
        
        hr = g_pTextService->Activate(pThreadMgr, clientId);
        if (FAILED(hr)) {
            printf("TextService激活失败: 0x%08X\n", hr);
            pThreadMgr->Deactivate();
            pThreadMgr->Release();
            delete g_pTextService;
            g_pTextService = nullptr;
            CoUninitialize();
            return -3;
        }
        
        pThreadMgr->Release();
        
        printf("TSF初始化成功，ClientId: %d\n", clientId);
        return 0;
    }
    
    int tsf_insert_text(const char* text) {
        if (!g_pTextService || !text) {
            return -1;
        }
        
        // 转换UTF-8到UTF-16
        int wlen = MultiByteToWideChar(CP_UTF8, 0, text, -1, nullptr, 0);
        if (wlen <= 0) {
            return -2;
        }
        
        WCHAR* wtext = new WCHAR[wlen];
        MultiByteToWideChar(CP_UTF8, 0, text, -1, wtext, wlen);
        
        // 临时：直接使用SendInput方法
        HRESULT hr = g_pTextService->InsertTextViaSendInput(wtext);
        
        delete[] wtext;
        
        return SUCCEEDED(hr) ? 0 : -3;
    }
    
    int tsf_cleanup() {
        if (g_pTextService) {
            g_pTextService->Release();
            g_pTextService = nullptr;
        }
        
        CoUninitialize();
        return 0;
    }
}

// EditSession 类实现
YiEditSession::YiEditSession(YiTextService *pTextService, const WCHAR *pszText) {
    m_cRef = 1;
    m_pTextService = pTextService;
    m_pTextService->AddRef();
    
    // 复制文本
    size_t len = wcslen(pszText) + 1;
    m_pszText = new WCHAR[len];
    wcscpy_s(m_pszText, len, pszText);
}

YiEditSession::~YiEditSession() {
    if (m_pTextService) {
        m_pTextService->Release();
    }
    if (m_pszText) {
        delete[] m_pszText;
    }
}

STDMETHODIMP YiEditSession::QueryInterface(REFIID riid, void **ppvObj) {
    if (ppvObj == nullptr) {
        return E_INVALIDARG;
    }
    
    *ppvObj = nullptr;
    
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfEditSession)) {
        *ppvObj = (ITfEditSession*)this;
    }
    
    if (*ppvObj) {
        AddRef();
        return S_OK;
    }
    
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) YiEditSession::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

STDMETHODIMP_(ULONG) YiEditSession::Release() {
    ULONG cRef = InterlockedDecrement(&m_cRef);
    if (cRef == 0) {
        delete this;
    }
    return cRef;
}

STDMETHODIMP YiEditSession::DoEditSession(TfEditCookie ec) {
    ITfContext *pContext = m_pTextService->GetContext();
    if (!pContext || !m_pszText) {
        printf("DoEditSession: 上下文或文本为空\n");
        return E_FAIL;
    }
    
    printf("DoEditSession: 开始插入文本: %ls\n", m_pszText);
    
    ITfInsertAtSelection *pInsertAtSelection = nullptr;
    ITfRange *pRange = nullptr;
    
    HRESULT hr = pContext->QueryInterface(IID_ITfInsertAtSelection, (void**)&pInsertAtSelection);
    if (FAILED(hr)) {
        printf("DoEditSession: 获取InsertAtSelection接口失败: 0x%08X\n", hr);
        return hr;
    }
    
    printf("DoEditSession: 成功获取InsertAtSelection接口\n");
    
    hr = pInsertAtSelection->InsertTextAtSelection(ec, 0, m_pszText, (LONG)wcslen(m_pszText), &pRange);
    printf("DoEditSession: InsertTextAtSelection返回: 0x%08X\n", hr);
    
    if (SUCCEEDED(hr)) {
        if (pRange) {
            printf("DoEditSession: 成功创建文本范围\n");
            pRange->Release();
        } else {
            printf("DoEditSession: 警告：文本范围为空\n");
        }
    } else {
        printf("DoEditSession: InsertTextAtSelection失败: 0x%08X\n", hr);
    }
    
    pInsertAtSelection->Release();
    
    return hr;
}