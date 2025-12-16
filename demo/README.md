# PenguinDemo: Julia Data Science Demo

Welcome to the demo project! This project showcases Julia's strengths in data analytics, math, and machine learning using the Palmer Penguins dataset.

## Project Structure

```
demo/
├── README.md                    # This file
├── penguin_analysis.ipynb      # Interactive EDA notebook
├── ml_training.ipynb           # ML training notebook
├── Project.toml                 # Julia project config
├── data/
│   └── penguins.csv            # Dataset (already included)
└── src/
    ├── data_loading.jl         # Data loading and preprocessing
    ├── ml_models.jl            # ML model implementations
    └── utils.jl                # General utility functions
```

## Files Overview

| File | Purpose | Description |
|------|---------|-------------|
| `penguin_analysis.ipynb` | EDA | Exploratory data analysis, visualizations, statistical tests, clustering, PCA |
| `ml_training.ipynb` | ML Training | Interactive ML pipeline: load data → train models → evaluate → compare |
| `src/data_loading.jl` | Data utilities | Functions for loading CSV, preprocessing, and feature preparation |
| `src/ml_models.jl` | ML implementations | Model training functions (NN, LR, KNN) and evaluation |
| `src/utils.jl` | General utilities | Helper functions for accuracy, predictions, summaries |

## Quick Start

### 1. Exploratory Data Analysis

Open and run `penguin_analysis.ipynb` to explore the dataset:
- Data loading and basic statistics
- Advanced visualizations (pair plots, violin plots)
- Statistical hypothesis testing
- Clustering analysis (K-means, hierarchical)
- Dimensionality reduction (PCA)
- Outlier detection

### 2. Machine Learning Training

Open and run `ml_training.ipynb` to train and compare ML models:
- Load and preprocess data using `src/data_loading.jl`
- Train three models:
  - Neural Network (Flux.jl)
  - Logistic Regression (Flux.jl)
  - K-Nearest Neighbors (NearestNeighbors.jl)
- Evaluate and compare model performance
- View sample predictions

## Features

- **Data loading and cleaning** with DataFrames.jl
- **Exploratory data analysis** and plotting with Plots.jl
- **Linear algebra and statistics** with built-in Julia packages
- **Multiple dispatch** for custom summaries
- **Machine learning** with Flux.jl and NearestNeighbors.jl
- **Modular design** with reusable functions in `src/` modules

## Dataset

We use the [Palmer Penguins dataset](https://github.com/allisonhorst/palmerpenguins) (CC0), a fun alternative to the classic Iris dataset. The dataset is already included in `data/penguins.csv`.

## Usage

### Running Notebooks

1. **EDA Notebook**: Start with `penguin_analysis.ipynb` to understand the data
2. **ML Training Notebook**: Then run `ml_training.ipynb` to train models

Both notebooks import functions from the `src/` modules, so you can also use these functions in your own code.

### Using Source Modules

You can import the modules in your own Julia code:

```julia
include("src/data_loading.jl")
include("src/ml_models.jl")
include("src/utils.jl")

# Load data
df = load_penguin_data()

# Preprocess
df_clean = preprocess_data(df, [:species, :body_mass_g])

# Train models
X_train, y_train, X_val, y_val, species_unique, species_dict = prepare_features(df_clean)
nn_model = train_neural_network(X_train, y_train, X_val, y_val, length(species_unique))
```

## Dependencies

See `Project.toml` for the complete list of dependencies. Key packages:
- DataFrames.jl - Data manipulation
- CSV.jl - CSV file reading
- Plots.jl - Plotting
- Flux.jl - Neural networks
- NearestNeighbors.jl - KNN implementation
- Statistics.jl - Statistical functions
