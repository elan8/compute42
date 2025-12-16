# PenguinDemo: Machine Learning Model Implementations

using Flux, NearestNeighbors, Random, Statistics, Printf

# Import utility functions (will be available when utils.jl is included)
# get_predictions, calculate_accuracy are defined in utils.jl

"""
Train a neural network for species classification.
Returns trained model and predictions.
"""
function train_neural_network(X_train, y_train, X_val, y_val, num_species::Int, epochs::Int = 20)
    println("\n" * "="^60)
    println("MODEL 1: Neural Network (Flux)")
    println("="^60)
    
    # Neural Network architecture
    nn_model = Chain(
        Dense(4, 16, relu),
        Dense(16, 8, relu),
        Dense(8, num_species),
        softmax
    )
    
    loss(model, x, y) = Flux.crossentropy(model(x), y)
    opt = Descent(0.01)
    
    println("Architecture: 4 → 16 → 8 → $num_species (ReLU + Softmax)")
    
    # Training loop
    for epoch in 1:epochs
        Flux.train!(loss, nn_model, [(X_train, y_train)], opt)
        
        if epoch % 5 == 0
            train_loss = loss(nn_model, X_train, y_train)
            val_loss = loss(nn_model, X_val, y_val)
            println("Epoch $epoch: Train Loss: $(round(train_loss, digits=3)), Val Loss: $(round(val_loss, digits=3))")
        end
    end
    
    return nn_model
end

"""
Train logistic regression using Flux approach.
Returns weights W and bias b.
"""
function train_logistic_regression(X_train, y_train, learning_rate::Float32 = 0.1f0, epochs::Int = 100)
    println("\n" * "="^60)
    println("MODEL 2: Logistic Regression (Flux)")
    println("="^60)
    
    n_features, n_samples = size(X_train)
    n_classes = size(y_train, 1)
    
    # Initialize weights and bias 
    W = rand(Float32, n_classes, n_features) * 0.01f0
    b = zeros(Float32, n_classes, 1)
    
    # Define model function 
    model(W, b, x) = W * x .+ b
    
    # Define custom softmax
    custom_softmax(x) = exp.(x) ./ sum(exp.(x), dims=1)
    
    # Define custom model with softmax 
    custom_model(W, b, x) = custom_softmax(model(W, b, x))
    
    # Define loss function 
    function custom_loss(W, b, x, y_onehot)
        predictions = custom_model(W, b, x)
        # Use Float32() constructor instead of f0 literal to avoid Zygote issues
        epsilon = Float32(1e-8)
        return -sum(y_onehot .* log.(predictions .+ epsilon)) / n_samples
    end
    
    # Training loop with gradient descent 
    println("Training logistic regression (Flux style)...")
    for epoch in 1:epochs
        # Get gradients using Flux's gradient function
        dLdW, dLdb, _, _ = gradient(custom_loss, W, b, X_train, y_train)
        
        # Update parameters
        W .= W .- learning_rate .* dLdW
        b .= b .- learning_rate .* dLdb
    end
    
    return W, b
end

"""
Predict using logistic regression model.
"""
function predict_lr_flux(X, W, b)
    custom_softmax(x) = exp.(x) ./ sum(exp.(x), dims=1)
    model(W, b, x) = W * x .+ b
    custom_model(W, b, x) = custom_softmax(model(W, b, x))
    return get_predictions(custom_model(W, b, X))
end

"""
Train K-Nearest Neighbors model.
Returns KDTree for predictions.
"""
function train_knn(X_train, y_train, num_species::Int, k::Int = 5)
    println("\n" * "="^60)
    println("MODEL 3: K-Nearest Neighbors (NearestNeighbors.jl)")
    println("="^60)
    
    # High-performance KNN using NearestNeighbors.jl
    println("Building KDTree for efficient nearest neighbor search...")
    
    # Build KDTree from training data
    kdtree = KDTree(X_train)
    
    return kdtree
end

"""
Predict using K-Nearest Neighbors.
"""
function predict_knn(tree, X_train, y_train, X_test, num_classes::Int, k::Int = 5)
    println("Making predictions with k=$k...")
    predictions = Int[]
    
    for i in 1:size(X_test, 2)
        # Find k nearest neighbors using KDTree
        idxs, dists = knn(tree, X_test[:, i], k)
        
        # Get labels of nearest neighbors
        neighbor_labels = [argmax(y_train[:, idx]) for idx in idxs]
        
        # Majority vote
        label_counts = Dict{Int, Int}()
        for label in neighbor_labels
            label_counts[label] = get(label_counts, label, 0) + 1
        end
        
        # Find most common label
        predicted_label = argmax([get(label_counts, j, 0) for j in 1:num_classes])
        push!(predictions, predicted_label)
    end
    
    return predictions
end

"""
Compare model results and return best model name and accuracy.
"""
function compare_models(results::Dict)
    println("\n" * "="^60)
    println("MODEL COMPARISON RESULTS")
    println("="^60)
    
    # Display comparison table
    println("Model                    | Train Acc | Val Acc  |")
    println("-"^50)
    for (model_name, (train_acc, val_acc)) in results
        println(@sprintf("%-25s | %8.1f%% | %7.1f%% |", model_name, train_acc*100, val_acc*100))
    end
    
    # Find best model
    best_model = argmax([val_acc for (_, val_acc) in values(results)])
    best_model_name = collect(keys(results))[best_model]
    best_val_acc = results[best_model_name][2]
    
    println("\n🏆 Best Model: $best_model_name (Validation Accuracy: $(round(best_val_acc*100, digits=1))%)")
    
    return best_model_name, best_val_acc
end

