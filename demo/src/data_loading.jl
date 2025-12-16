# PenguinDemo: Data Loading and Preprocessing

using DataFrames, CSV, Random, Statistics

"""
Load the penguin dataset from CSV file.
Returns a DataFrame with proper column types and missing value handling.
"""
function load_penguin_data(data_path::String = joinpath(@__DIR__, "..", "data", "penguins.csv"))
    df = CSV.read(data_path, DataFrame, 
        types=Dict(
            :bill_length_mm => Union{Float64, Missing},
            :bill_depth_mm => Union{Float64, Missing},
            :flipper_length_mm => Union{Int64, Missing},
            :body_mass_g => Union{Int64, Missing},
            :year => Int64
        ),
        missingstring="NA"
    )
    println("Loaded $(size(df, 1)) penguin records from Palmer Station LTER!")
    return df
end

"""
Preprocess data by dropping missing values for specified columns.
Returns cleaned DataFrame.
"""
function preprocess_data(df::DataFrame, required_cols::Vector{Symbol})
    df_clean = dropmissing(df, required_cols)
    return df_clean
end

"""
Prepare features for machine learning.
Returns normalized feature matrix X, one-hot encoded labels y, 
species mapping dictionary, and train/validation splits.
"""
function prepare_features(df_clean::DataFrame, numeric_cols::Vector{Symbol} = [:bill_length_mm, :bill_depth_mm, :flipper_length_mm, :body_mass_g])
    # Normalize features
    X_raw = Matrix(select(df_clean, numeric_cols))
    X_mean = mean(X_raw, dims=1)
    X_std = std(X_raw, dims=1)
    X = Float32.((X_raw .- X_mean) ./ X_std)'
    
    # Create one-hot encoding manually
    species_unique = unique(df_clean.species)
    species_dict = Dict(species => i for (i, species) in enumerate(species_unique))
    y_indices = [species_dict[species] for species in df_clean.species]
    y = zeros(Float32, length(species_unique), length(y_indices))
    for (i, idx) in enumerate(y_indices)
        y[idx, i] = 1.0f0
    end
    
    # Split data into training and validation sets (80/20 split)
    n_samples = size(X, 2)
    n_train = Int(floor(0.8 * n_samples))
    indices = shuffle(1:n_samples)
    train_indices = indices[1:n_train]
    val_indices = indices[n_train+1:end]
    
    X_train = X[:, train_indices]
    y_train = y[:, train_indices]
    X_val = X[:, val_indices]
    y_val = y[:, val_indices]
    
    println("Dataset split:")
    println("  Training samples: $(size(X_train, 2))")
    println("  Validation samples: $(size(X_val, 2))")
    println("  Features: $(size(X_train, 1))")
    println("  Species: $(length(species_unique))")
    
    return X_train, y_train, X_val, y_val, species_unique, species_dict
end

