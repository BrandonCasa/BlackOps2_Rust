#include "ContentPrinter.h"
#include "ObjContainer/IPak/IPak.h"
#include "Image/IwiLoader.h"
#include "Image/DdsWriter.h"
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <stdexcept>

ContentPrinter::ContentPrinter(const Zone& zone) : m_zone(zone) {}
void ContentPrinter::PrintContent() const {
    const char* data_root = std::getenv("ORIGINS_DATA_ROOT");
    const char* output_root = std::getenv("ORIGINS_IMAGE_OUTPUT");
    if (!data_root || !output_root) throw std::runtime_error("missing external package paths");
    for (const auto& file : std::filesystem::directory_iterator(data_root)) {
        if (!file.is_regular_file() || file.path().extension() != ".ipak") continue;
        auto stream = std::make_unique<std::ifstream>(file.path(), std::ios::binary);
        auto package = IIPak::Create(file.path().filename().string(), std::move(stream));
        if (!package->Initialize()) throw std::runtime_error("invalid image package");
        const auto folder = std::filesystem::path(output_root) / file.path().stem();
        std::filesystem::create_directories(folder);
        std::ofstream index(folder / "entries.json");
        index << '[';
        bool first = true;
        unsigned decoded = 0;
        for (const auto& entry : package->GetIndexEntries()) {
            std::ostringstream name;
            name << std::hex << std::setfill('0') << std::setw(8) << entry.key.nameHash
                 << '-' << std::setw(8) << entry.key.dataHash;
            const auto original = folder / (name.str() + ".iwi");
            if (!std::filesystem::is_regular_file(original)) {
                auto image = package->GetEntryStream(entry.key.nameHash, entry.key.dataHash);
                if (!image) throw std::runtime_error("missing indexed package entry");
                std::ofstream output(original, std::ios::binary);
                output << image->rdbuf();
                image->close();
                if (!output.good()) throw std::runtime_error("package entry retention failed");
            }
            bool prepared = std::filesystem::is_regular_file(folder / (name.str() + ".dds"));
            if (!prepared) {
                std::ifstream input(original, std::ios::binary);
                auto image = image::LoadIwi(input);
                image::DdsWriter writer;
                if (image && writer.SupportsImageFormat(image->m_texture->GetFormat())) {
                    std::ofstream output(folder / (name.str() + ".dds"), std::ios::binary);
                    writer.DumpImage(output, image->m_texture.get());
                    if (!output.good()) throw std::runtime_error("package DDS retention failed");
                    prepared = true;
                }
            }
            if (!first) index << ',';
            first = false;
            index << "{\"name_hash\":" << entry.key.nameHash << ",\"data_hash\":" << entry.key.dataHash
                  << ",\"identity\":" << std::quoted(name.str()) << ",\"decoded\":" << (prepared ? "true" : "false") << '}';
            decoded += prepared ? 1u : 0u;
        }
        index << ']';
        if (!index.good()) throw std::runtime_error("package index write failed");
        std::cout << file.path().filename().string() << ": " << decoded << '/' << package->GetIndexEntries().size() << " entries decoded" << std::endl;
    }
}
