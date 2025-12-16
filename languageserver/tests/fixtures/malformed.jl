function broken_function(
    # Missing closing parenthesis
    x::Int
    return x + 1
end

# Missing 'end' keyword
if true
    println("This will cause a parse error")

# Invalid syntax
let x = 5
    y = x + 1
# Missing 'end'
