#pragma once
#include <windows.h>
#include <msctf.h>
#include <ctfutb.h>
#include <olectl.h>
#include <comdef.h>

// GUID定义
EXTERN_C const GUID CLSID_YiTextService;
EXTERN_C const GUID GUID_YiProfile;

// 前向声明
class YiTextService;

// EditSession 类用于文本插入
class YiEditSession : public ITfEditSession {
public:
    YiEditSession(YiTextService *pTextService, const WCHAR *pszText);
    ~YiEditSession();

    // IUnknown接口
    STDMETHODIMP QueryInterface(REFIID riid, void **ppvObj);
    STDMETHODIMP_(ULONG) AddRef();
    STDMETHODIMP_(ULONG) Release();

    // ITfEditSession接口
    STDMETHODIMP DoEditSession(TfEditCookie ec);

private:
    LONG m_cRef;
    YiTextService *m_pTextService;
    WCHAR *m_pszText;
};

// 导出的C接口函数
extern "C" {
    __declspec(dllexport) int tsf_initialize();
    __declspec(dllexport) int tsf_insert_text(const char* text);
    __declspec(dllexport) int tsf_cleanup();
    __declspec(dllexport) BOOL DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved);
}

// TSF文本服务类
class YiTextService : public ITfTextInputProcessor,
                      public ITfThreadMgrEventSink,
                      public ITfTextEditSink,
                      public ITfKeyEventSink {
public:
    YiTextService();
    ~YiTextService();

    // IUnknown接口
    STDMETHODIMP QueryInterface(REFIID riid, void **ppvObj);
    STDMETHODIMP_(ULONG) AddRef();
    STDMETHODIMP_(ULONG) Release();

    // ITfTextInputProcessor接口
    STDMETHODIMP Activate(ITfThreadMgr *pThreadMgr, TfClientId tfClientId);
    STDMETHODIMP Deactivate();

    // ITfThreadMgrEventSink接口
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr *pDocMgr);
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr *pDocMgr);
    STDMETHODIMP OnSetFocus(ITfDocumentMgr *pDocMgrFocus, ITfDocumentMgr *pDocMgrPrevFocus);
    STDMETHODIMP OnPushContext(ITfContext *pContext);
    STDMETHODIMP OnPopContext(ITfContext *pContext);

    // ITfTextEditSink接口
    STDMETHODIMP OnEndEdit(ITfContext *pContext, TfEditCookie ecReadOnly, ITfEditRecord *pEditRecord);

    // ITfKeyEventSink接口
    STDMETHODIMP OnSetFocus(BOOL fForeground);
    STDMETHODIMP OnTestKeyDown(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten);
    STDMETHODIMP OnKeyDown(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten);
    STDMETHODIMP OnTestKeyUp(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten);
    STDMETHODIMP OnKeyUp(ITfContext *pContext, WPARAM wParam, LPARAM lParam, BOOL *pfEaten);
    STDMETHODIMP OnPreservedKey(ITfContext *pContext, REFGUID rguid, BOOL *pfEaten);

    // 文本插入方法
    HRESULT InsertText(const WCHAR *pszText);
    
    // 备选文本插入方法
    HRESULT InsertTextViaSendInput(const WCHAR *pszText);
    
    // 获取当前上下文（供 EditSession 使用）
    ITfContext* GetContext() { return m_pContext; }

private:
    LONG m_cRef;
    ITfThreadMgr *m_pThreadMgr;
    TfClientId m_tfClientId;
    ITfContext *m_pContext;
    DWORD m_dwThreadMgrEventSinkCookie;
    DWORD m_dwTextEditSinkCookie;
    DWORD m_dwKeyEventSinkCookie;

    HRESULT _InitThreadMgrSink();
    HRESULT _UninitThreadMgrSink();
    HRESULT _InitTextEditSink(ITfDocumentMgr *pDocMgr);
    HRESULT _UninitTextEditSink();
    HRESULT _InitKeyEventSink();
    HRESULT _UninitKeyEventSink();
    HRESULT _GetFocusContext();  // 新增方法
};

// 全局变量
extern YiTextService* g_pTextService;