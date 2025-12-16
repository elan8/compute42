# PenguinDemo: General Utility Functions

using DataFrames, Statistics

# Multiple dispatch: custom summary
function penguin_summary(df::DataFrame)
    println("Species: ", unique(df.species))
    println("Islands: ", unique(df.island))
end

function penguin_summary(df::DataFrame, col::Symbol)
    println("Mean $(col): ", mean(skipmissing(df[!, col])))
end

# Function to calculate accuracy
function calculate_accuracy(predictions, true_labels)
    return mean(predictions .== true_labels)
end

# Function to get predictions from one-hot encoded output
function get_predictions(model_output)
    return [argmax(model_output[:, i]) for i in 1:size(model_output, 2)]
end

