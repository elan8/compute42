"""
A macro that doubles its input.
"""
macro double(x)
    return :(2 * $x)
end

"""
A macro with documentation.
"""
macro documented_macro(expr)
    quote
        println("Executing: ", $(string(expr)))
        $expr
    end
end
