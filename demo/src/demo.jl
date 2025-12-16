# PenguinDemo: Main Module
# This is the main entry point for the demo package

module demo

# Include all submodules
include("data_loading.jl")
include("utils.jl")
include("ml_models.jl")

# Export main functions for easy access
export load_penguin_data
export preprocess_data
export prepare_features
export penguin_summary
export calculate_accuracy
export get_predictions
export train_neural_network
export train_logistic_regression
export predict_lr_flux
export train_knn
export predict_knn
export compare_models

end # module demo

