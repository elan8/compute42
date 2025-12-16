# Extract Base/stdlib documentation 
# This script extracts documentation for all Base, Core, and stdlib symbols
# and stores them in a JSON file for use by the languageserver

using JSON
using Pkg

# Get output path from command line argument or use default
output_path = length(ARGS) > 0 ? ARGS[1] : "base_docs.json"

println(stderr, "Extracting Base/stdlib documentation...")

# Dictionary to store all documentation
docs_dict = Dict{String, String}()

# Function to safely get documentation
function safe_get_docs(binding)
    try
        doc = Docs.doc(binding)
        if doc !== nothing
            if isa(doc, Markdown.MD)
                io = IOBuffer()
                Markdown.plain(io, doc)
                content = String(take!(io))
                return strip(content)
            else
                return strip(string(doc))
            end
        end
    catch e
        # Silently skip errors
    end
    return nothing
end

# Function to extract docs from a module
function extract_module_docs(module_name, mod)
    count = 0
    try
        for name in names(mod; all=true)
            # Skip internal names starting with #
            if startswith(string(name), "#")
                continue
            end
            
            try
                binding = Base.Docs.Binding(mod, name)
                doc_content = safe_get_docs(binding)
                
                if doc_content !== nothing && !isempty(strip(doc_content))
                    # Filter out error messages
                    content_lower = lowercase(doc_content)
                    is_error = contains(content_lower, "no documentation found") ||
                              (contains(content_lower, "binding") && contains(content_lower, "does not exist"))
                    
                    if !is_error
                        symbol_key = string(name)
                        docs_dict[symbol_key] = doc_content
                        count += 1
                    end
                end
            catch e
                # Skip symbols that can't be accessed
                continue
            end
        end
    catch e
        println(stderr, "Error extracting docs from $module_name: ", e)
    end
    return count
end

# Extract from Base
println(stderr, "Extracting Base documentation...")
base_count = extract_module_docs("Base", Base)
println(stderr, "  Extracted $base_count symbols from Base")

# Extract from Core
println(stderr, "Extracting Core documentation...")
core_count = extract_module_docs("Core", Core)
println(stderr, "  Extracted $core_count symbols from Core")

# Extract from stdlib modules
println(stderr, "Extracting stdlib documentation...")
stdlib_modules = [
    "Dates", "DelimitedFiles", "Distributed", "FileWatching", "Future",
    "InteractiveUtils", "Libdl", "LibGit2", "LinearAlgebra", "Logging",
    "Markdown", "Mmap", "Pkg", "Printf", "Profile", "Random", "REPL",
    "Serialization", "SHA", "Sockets", "SparseArrays", "Statistics",
    "SuiteSparse", "Test", "UUIDs", "Unicode"
]

stdlib_total = 0
for mod_name in stdlib_modules
    try
        mod = getfield(Main, Symbol(mod_name))
        count = extract_module_docs(mod_name, mod)
        stdlib_total += count
        if count > 0
            println(stderr, "  Extracted $count symbols from $mod_name")
        end
    catch e
        # Module not available, skip
        continue
    end
end

println(stderr, "  Extracted $stdlib_total symbols from stdlib modules")

# Write to JSON file
println(stderr, "Writing documentation to $output_path...")
open(output_path, "w") do f
    JSON.print(f, docs_dict, 2)
end

total_symbols = length(docs_dict)
file_size = filesize(output_path)
println(stderr, "Extraction complete: $total_symbols symbols, $(round(file_size / 1024, digits=2)) KB")

















