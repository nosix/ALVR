#include "amf_helper.h"

#include <dlfcn.h>
#include <fcntl.h>
#include <unistd.h>

namespace alvr
{

AMFContext::AMFContext()
{
    init();
}

bool AMFContext::isValid() const
{
    return m_valid;
}

amf::AMFFactory *AMFContext::factory() const
{
    return m_factory;
}

amf::AMFContextPtr AMFContext::context() const
{
    return m_context;
}

std::vector<const char*> AMFContext::requiredDeviceExtensions() const
{
    if (!m_context1) {
        return {};
    }
    size_t count;
    m_context1->GetVulkanDeviceExtensions(&count, nullptr);
    std::vector<const char*> out(count);
    m_context1->GetVulkanDeviceExtensions(&count, out.data());
    return out;
}

void AMFContext::initialize(amf::AMFVulkanDevice *dev)
{
    if (!m_context1) {
        throw "No Context1";
    }

    bool ok = m_context1->InitVulkan(dev) == AMF_OK;

    unsetenv("VK_DRIVER_FILES");
    unsetenv("VK_ICD_FILENAMES");

    if (!ok) {
        throw "Failed to initialize Vulkan AMF";
    }
}

AMFContext *AMFContext::get()
{
    static AMFContext *s = nullptr;
    if (!s) {
        s = new AMFContext;
    }
    return s;
}

void AMFContext::init()
{
    void *amf_module = dlopen(AMF_DLL_NAMEA, RTLD_LAZY);
    if (!amf_module) {
        return;
    }

    auto init = (AMFInit_Fn)dlsym(amf_module, AMF_INIT_FUNCTION_NAME);
    if (!init) {
        return;
    }

    if (init(AMF_FULL_VERSION, &m_factory) != AMF_OK) {
        return;
    }

    if (m_factory->CreateContext(&m_context) != AMF_OK) {
        return;
    }

    m_context1 = amf::AMFContext1Ptr(m_context);

    char *vk_icd_file = getenv("ALVR_AMF_ICD");
    if (!vk_icd_file || access(vk_icd_file, F_OK) != 0) {
        return;
    }

    setenv("VK_DRIVER_FILES", vk_icd_file, 1);
    setenv("VK_ICD_FILENAMES", vk_icd_file, 1);

    m_valid = true;
}

};
