module TestModule

"""
A module-level function.
"""
function module_function(x)
    return x * 2
end

"""
Another function in the module.
"""
function another_function(y::Int)
    return y + 1
end

# Module-level variable
const MODULE_CONSTANT = 100

end
