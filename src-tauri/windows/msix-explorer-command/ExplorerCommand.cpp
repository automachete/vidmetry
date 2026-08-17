#include <windows.h>

#include <appmodel.h>
#include <new>
#include <shellapi.h>
#include <shlobj_core.h>
#include <shlwapi.h>
#include <strsafe.h>

namespace {

constexpr CLSID kCommandClsid = {
    0x7cd16804,
    0x1388,
    0x4150,
    {0x99, 0x1b, 0xa9, 0x77, 0xae, 0xa2, 0x25, 0x67},
};
constexpr wchar_t kDisplayName[] = L"Open with Vidmetry";
constexpr wchar_t kStateKey[] = L"Software\\Vidmetry";
constexpr wchar_t kStateValue[] = L"ExplorerIntegrationEnabled";

HINSTANCE g_module = nullptr;
long g_moduleReferences = 0;

HRESULT CopyString(const wchar_t* value, wchar_t** output) {
    if (output == nullptr) {
        return E_POINTER;
    }
    *output = nullptr;
    return SHStrDupW(value, output);
}

HRESULT GetVidmetryPath(wchar_t (&path)[MAX_PATH]) {
    const DWORD length = GetModuleFileNameW(g_module, path, ARRAYSIZE(path));
    if (length == 0 || length == ARRAYSIZE(path)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    if (!PathRemoveFileSpecW(path) || !PathAppendW(path, L"vidmetry.exe")) {
        return E_FAIL;
    }
    return S_OK;
}

bool IsIntegrationEnabled() {
    DWORD enabled = 1;
    DWORD size = sizeof(enabled);
    const LSTATUS status = RegGetValueW(
        HKEY_CURRENT_USER,
        kStateKey,
        kStateValue,
        RRF_RT_REG_DWORD,
        nullptr,
        &enabled,
        &size);
    return status == ERROR_FILE_NOT_FOUND || (status == ERROR_SUCCESS && enabled != 0);
}

HRESULT LaunchVidmetry(const wchar_t* selectedPath) {
    size_t selectedLength = 0;
    HRESULT result = StringCchLengthW(selectedPath, 32765, &selectedLength);
    if (FAILED(result)) {
        return result;
    }
    const size_t parameterLength = selectedLength + 3;
    auto* parameters = static_cast<wchar_t*>(
        CoTaskMemAlloc(parameterLength * sizeof(wchar_t)));
    if (parameters == nullptr) {
        return E_OUTOFMEMORY;
    }
    result = StringCchPrintfW(parameters, parameterLength, L"\"%s\"", selectedPath);
    if (FAILED(result)) {
        CoTaskMemFree(parameters);
        return result;
    }

    UINT32 familyLength = PACKAGE_FAMILY_NAME_MAX_LENGTH + 1;
    wchar_t family[PACKAGE_FAMILY_NAME_MAX_LENGTH + 1] = {};
    const LONG familyResult = GetCurrentPackageFamilyName(&familyLength, family);
    if (familyResult == ERROR_SUCCESS) {
        wchar_t applicationUserModelId[APPLICATION_USER_MODEL_ID_MAX_LENGTH] = {};
        result = StringCchPrintfW(
            applicationUserModelId,
            ARRAYSIZE(applicationUserModelId),
            L"%s!Vidmetry",
            family);
        if (FAILED(result)) {
            CoTaskMemFree(parameters);
            return result;
        }

        IApplicationActivationManager* activationManager = nullptr;
        result = CoCreateInstance(
            CLSID_ApplicationActivationManager,
            nullptr,
            CLSCTX_INPROC_SERVER,
            IID_PPV_ARGS(&activationManager));
        if (SUCCEEDED(result)) {
            DWORD processId = 0;
            result = activationManager->ActivateApplication(
                applicationUserModelId, parameters, AO_NONE, &processId);
            activationManager->Release();
            if (SUCCEEDED(result)) {
                CoTaskMemFree(parameters);
                return result;
            }
        }
    }

    wchar_t executable[MAX_PATH] = {};
    result = GetVidmetryPath(executable);
    if (FAILED(result)) {
        CoTaskMemFree(parameters);
        return result;
    }
    const HINSTANCE launchResult = ShellExecuteW(
        nullptr, L"open", executable, parameters, nullptr, SW_SHOWNORMAL);
    CoTaskMemFree(parameters);
    const auto code = reinterpret_cast<INT_PTR>(launchResult);
    return code > 32 ? S_OK : HRESULT_FROM_WIN32(static_cast<DWORD>(code));
}

class ExplorerCommand final : public IExplorerCommand {
public:
    ExplorerCommand() noexcept : references_(1) {
        InterlockedIncrement(&g_moduleReferences);
    }

    ExplorerCommand(const ExplorerCommand&) = delete;
    ExplorerCommand& operator=(const ExplorerCommand&) = delete;

    IFACEMETHODIMP QueryInterface(REFIID interfaceId, void** object) override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        if (interfaceId == IID_IUnknown || interfaceId == IID_IExplorerCommand) {
            *object = static_cast<IExplorerCommand*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    IFACEMETHODIMP_(ULONG) AddRef() override {
        return static_cast<ULONG>(InterlockedIncrement(&references_));
    }

    IFACEMETHODIMP_(ULONG) Release() override {
        const long references = InterlockedDecrement(&references_);
        if (references == 0) {
            delete this;
        }
        return static_cast<ULONG>(references);
    }

    IFACEMETHODIMP GetTitle(IShellItemArray*, wchar_t** title) override {
        return CopyString(kDisplayName, title);
    }

    IFACEMETHODIMP GetIcon(IShellItemArray*, wchar_t** icon) override {
        if (icon == nullptr) {
            return E_POINTER;
        }
        *icon = nullptr;
        wchar_t executable[MAX_PATH] = {};
        const HRESULT pathResult = GetVidmetryPath(executable);
        if (FAILED(pathResult)) {
            return pathResult;
        }
        wchar_t iconReference[MAX_PATH + 3] = {};
        const HRESULT formatResult = StringCchPrintfW(
            iconReference, ARRAYSIZE(iconReference), L"%s,0", executable);
        return SUCCEEDED(formatResult) ? CopyString(iconReference, icon) : formatResult;
    }

    IFACEMETHODIMP GetToolTip(IShellItemArray*, wchar_t** toolTip) override {
        return CopyString(L"Open the selected folder in Vidmetry", toolTip);
    }

    IFACEMETHODIMP GetCanonicalName(GUID* commandName) override {
        if (commandName == nullptr) {
            return E_POINTER;
        }
        *commandName = kCommandClsid;
        return S_OK;
    }

    IFACEMETHODIMP GetState(IShellItemArray*, BOOL, EXPCMDSTATE* state) override {
        if (state == nullptr) {
            return E_POINTER;
        }
        *state = IsIntegrationEnabled() ? ECS_ENABLED : ECS_HIDDEN;
        return S_OK;
    }

    IFACEMETHODIMP Invoke(IShellItemArray* items, IBindCtx*) override {
        if (items == nullptr) {
            return E_INVALIDARG;
        }
        IShellItem* item = nullptr;
        HRESULT result = items->GetItemAt(0, &item);
        if (FAILED(result)) {
            return result;
        }

        wchar_t* selectedPath = nullptr;
        result = item->GetDisplayName(SIGDN_FILESYSPATH, &selectedPath);
        item->Release();
        if (FAILED(result)) {
            return result;
        }

        result = LaunchVidmetry(selectedPath);
        CoTaskMemFree(selectedPath);
        return result;
    }

    IFACEMETHODIMP GetFlags(EXPCMDFLAGS* flags) override {
        if (flags == nullptr) {
            return E_POINTER;
        }
        *flags = ECF_DEFAULT;
        return S_OK;
    }

    IFACEMETHODIMP EnumSubCommands(IEnumExplorerCommand** commands) override {
        if (commands != nullptr) {
            *commands = nullptr;
        }
        return E_NOTIMPL;
    }

private:
    ~ExplorerCommand() {
        InterlockedDecrement(&g_moduleReferences);
    }

    long references_;
};

class ClassFactory final : public IClassFactory {
public:
    ClassFactory() noexcept : references_(1) {
        InterlockedIncrement(&g_moduleReferences);
    }

    ClassFactory(const ClassFactory&) = delete;
    ClassFactory& operator=(const ClassFactory&) = delete;

    IFACEMETHODIMP QueryInterface(REFIID interfaceId, void** object) override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        if (interfaceId == IID_IUnknown || interfaceId == IID_IClassFactory) {
            *object = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    IFACEMETHODIMP_(ULONG) AddRef() override {
        return static_cast<ULONG>(InterlockedIncrement(&references_));
    }

    IFACEMETHODIMP_(ULONG) Release() override {
        const long references = InterlockedDecrement(&references_);
        if (references == 0) {
            delete this;
        }
        return static_cast<ULONG>(references);
    }

    IFACEMETHODIMP CreateInstance(IUnknown* outer, REFIID interfaceId, void** object) override {
        if (outer != nullptr) {
            return CLASS_E_NOAGGREGATION;
        }
        auto* command = new (std::nothrow) ExplorerCommand();
        if (command == nullptr) {
            return E_OUTOFMEMORY;
        }
        const HRESULT result = command->QueryInterface(interfaceId, object);
        command->Release();
        return result;
    }

    IFACEMETHODIMP LockServer(BOOL lock) override {
        if (lock) {
            InterlockedIncrement(&g_moduleReferences);
        } else {
            InterlockedDecrement(&g_moduleReferences);
        }
        return S_OK;
    }

private:
    ~ClassFactory() {
        InterlockedDecrement(&g_moduleReferences);
    }

    long references_;
};

}  // namespace

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, void*) {
    if (reason == DLL_PROCESS_ATTACH) {
        g_module = instance;
        DisableThreadLibraryCalls(instance);
    }
    return TRUE;
}

extern "C" HRESULT __stdcall DllCanUnloadNow() {
    return g_moduleReferences == 0 ? S_OK : S_FALSE;
}

extern "C" HRESULT __stdcall DllGetClassObject(
    REFCLSID classId, REFIID interfaceId, void** object) {
    if (classId != kCommandClsid) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    auto* factory = new (std::nothrow) ClassFactory();
    if (factory == nullptr) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = factory->QueryInterface(interfaceId, object);
    factory->Release();
    return result;
}
