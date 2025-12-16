# Compute42 JSON Utilities
# Safe JSON value extraction utilities that handle JSON.Object, JSON.Array, and primitive types consistently

# Extract a string value from JSON, handling all possible JSON types
# Returns Union{String, Nothing} - returns nothing if extraction fails
function extract_string(value)::Union{String, Nothing}
    if value === nothing
        return nothing
    elseif isa(value, String)
        return value
    elseif isa(value, JSON.Object)
        # JSON.Object is a Dict-like structure
        # Try to extract the value if it's a single-key object
        try
            keys_list = collect(keys(value))
            vals_list = collect(values(value))
            
            if length(keys_list) == 1 && length(vals_list) == 1
                # Single key-value pair - recursively extract the value
                return extract_string(vals_list[1])
            else
                # Multiple keys or empty - this shouldn't be a string field
                # Return nothing to indicate extraction failure
                return nothing
            end
        catch
            # If anything fails, return nothing
            return nothing
        end
    elseif isa(value, Number)
        # Convert number to string
        return string(value)
    elseif isa(value, Bool)
        # Convert bool to string
        return string(value)
    else
        # Try string conversion as last resort
        try
            return string(value)
        catch
            return nothing
        end
    end
end

# Extract an integer value from JSON
# Returns Union{Int, Nothing} - returns nothing if extraction fails
function extract_int(value)::Union{Int, Nothing}
    if value === nothing
        return nothing
    elseif isa(value, Int)
        return value
    elseif isa(value, Number)
        # Convert to Int (truncates floats)
        try
            return Int(value)
        catch
            return nothing
        end
    elseif isa(value, JSON.Object)
        # Try to extract from single-key object
        try
            keys_list = collect(keys(value))
            vals_list = collect(values(value))
            
            if length(keys_list) == 1 && length(vals_list) == 1
                return extract_int(vals_list[1])
            else
                return nothing
            end
        catch
            return nothing
        end
    elseif isa(value, String)
        # Try to parse string as integer
        try
            return parse(Int, value)
        catch
            return nothing
        end
    else
        return nothing
    end
end

# Extract a boolean value from JSON
# Returns Union{Bool, Nothing} - returns nothing if extraction fails
function extract_bool(value)::Union{Bool, Nothing}
    if value === nothing
        return nothing
    elseif isa(value, Bool)
        return value
    elseif isa(value, JSON.Object)
        # Try to extract from single-key object
        try
            keys_list = collect(keys(value))
            vals_list = collect(values(value))
            
            if length(keys_list) == 1 && length(vals_list) == 1
                return extract_bool(vals_list[1])
            else
                return nothing
            end
        catch
            return nothing
        end
    elseif isa(value, String)
        # Try to parse string as boolean
        lower_val = lowercase(value)
        if lower_val == "true"
            return true
        elseif lower_val == "false"
            return false
        else
            return nothing
        end
    else
        return nothing
    end
end

# Extract a dictionary from JSON
# Returns Union{Dict, Nothing} - returns nothing if extraction fails
function extract_dict(value)::Union{Dict, Nothing}
    if value === nothing
        return nothing
    elseif isa(value, Dict)
        return value
    elseif isa(value, JSON.Object)
        # Convert JSON.Object to Dict
        try
            return Dict{String, Any}(value)
        catch
            return nothing
        end
    else
        return nothing
    end
end

# Extract an array from JSON
# Returns Union{Array, Nothing} - returns nothing if extraction fails
function extract_array(value)::Union{Array, Nothing}
    if value === nothing
        return nothing
    elseif isa(value, Array)
        return value
    elseif isa(value, JSON.Array)
        # Convert JSON.Array to Array
        try
            return collect(value)
        catch
            return nothing
        end
    else
        return nothing
    end
end

# Extract a required string field from a data dictionary or JSON.Object
# Throws an error if the field is missing or cannot be extracted
function extract_required_string(data, field_name::String)::String
    # Handle both Dict and JSON.Object
    data_dict = isa(data, JSON.Object) ? Dict{String, Any}(data) : data
    
    if !haskey(data_dict, field_name)
        error("Missing required field: $(field_name)")
    end
    
    value = extract_string(data_dict[field_name])
    if value === nothing
        error("Field '$(field_name)' cannot be extracted as a string")
    end
    
    return value
end

# Extract an optional string field from a data dictionary or JSON.Object
# Returns nothing if the field is missing or cannot be extracted
function extract_optional_string(data, field_name::String)::Union{String, Nothing}
    # Handle both Dict and JSON.Object
    data_dict = isa(data, JSON.Object) ? Dict{String, Any}(data) : data
    
    if !haskey(data_dict, field_name)
        return nothing
    end
    
    return extract_string(data_dict[field_name])
end

# Extract a required integer field from a data dictionary or JSON.Object
# Throws an error if the field is missing or cannot be extracted
function extract_required_int(data, field_name::String)::Int
    # Handle both Dict and JSON.Object
    data_dict = isa(data, JSON.Object) ? Dict{String, Any}(data) : data
    
    if !haskey(data_dict, field_name)
        error("Missing required field: $(field_name)")
    end
    
    value = extract_int(data_dict[field_name])
    if value === nothing
        error("Field '$(field_name)' cannot be extracted as an integer")
    end
    
    return value
end

# Extract an optional integer field from a data dictionary or JSON.Object
# Returns nothing if the field is missing or cannot be extracted
function extract_optional_int(data, field_name::String)::Union{Int, Nothing}
    # Handle both Dict and JSON.Object
    data_dict = isa(data, JSON.Object) ? Dict{String, Any}(data) : data
    
    if !haskey(data_dict, field_name)
        return nothing
    end
    
    return extract_int(data_dict[field_name])
end

