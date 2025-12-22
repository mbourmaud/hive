# Example: Python ML Project

This example shows how to use Hive for machine learning projects with parallel experimentation.

## Project Structure

```
ml-project/
├── data/
│   ├── raw/           # Original datasets
│   ├── processed/     # Cleaned data
│   └── features/      # Feature engineering
├── models/
│   ├── baseline/
│   ├── experiments/
│   └── production/
├── notebooks/         # Jupyter notebooks
├── src/
│   ├── data/          # Data processing
│   ├── features/      # Feature engineering
│   ├── models/        # Model implementations
│   └── evaluation/    # Metrics and evaluation
└── pyproject.toml
```

## Setup

1. **Configure Hive for Python:**

```bash
# .env
WORKSPACE_NAME=ml-project
HIVE_DOCKERFILE=docker/Dockerfile.python
GIT_REPO_URL=https://github.com/user/ml-project.git
```

2. **Start Hive with 4 workers:**

```bash
hive init --workspace ml-project --workers 4 --no-interactive
```

## Example Workflow: Customer Churn Prediction

### Step 1: Queen Plans Experiments

Connect to Queen:
```bash
hive connect queen
```

Tell Queen:
```
Build a customer churn prediction model:
- Try 4 different algorithms in parallel
- Each algorithm should be tuned and evaluated
- Track all experiments with MLflow
```

Queen creates tasks:
```bash
hive-assign drone-1 "Train Logistic Regression baseline" "Train LR with hyperparameter tuning, track with MLflow" "ML-101"
hive-assign drone-2 "Train Random Forest model" "Train RF with grid search, track experiments" "ML-102"
hive-assign drone-3 "Train XGBoost model" "Train XGB with Optuna tuning, track experiments" "ML-103"
hive-assign drone-4 "Train Neural Network" "Train TensorFlow model with early stopping" "ML-104"
```

### Step 2: Workers Train Models in Parallel

**Terminal 2 - Drone 1 (Logistic Regression):**
```bash
hive connect 1
take-task
```

```python
# src/models/logistic_regression.py
import mlflow
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import GridSearchCV

mlflow.set_experiment("customer-churn")

with mlflow.start_run(run_name="logistic-regression-baseline"):
    # Log parameters
    mlflow.log_param("model", "LogisticRegression")
    mlflow.log_param("solver", "lbfgs")

    # Load data
    X_train, y_train = load_training_data()
    X_test, y_test = load_test_data()

    # Hyperparameter tuning
    param_grid = {
        'C': [0.001, 0.01, 0.1, 1, 10],
        'penalty': ['l2'],
        'max_iter': [100, 200, 300]
    }

    grid_search = GridSearchCV(
        LogisticRegression(),
        param_grid,
        cv=5,
        scoring='roc_auc',
        n_jobs=-1
    )
    grid_search.fit(X_train, y_train)

    # Best model
    best_model = grid_search.best_estimator_
    mlflow.log_params(grid_search.best_params_)

    # Evaluate
    y_pred = best_model.predict_proba(X_test)[:, 1]
    auc = roc_auc_score(y_test, y_pred)

    # Log metrics
    mlflow.log_metric("auc", auc)
    mlflow.log_metric("accuracy", accuracy_score(y_test, y_pred > 0.5))
    mlflow.log_metric("f1", f1_score(y_test, y_pred > 0.5))

    # Save model
    mlflow.sklearn.log_model(best_model, "model")

    print(f"AUC: {auc:.4f}")
```

Tests:
```bash
pytest tests/models/test_logistic_regression.py -v
```

When done:
```bash
task-done
```

**Terminal 3 - Drone 2 (Random Forest):**
```bash
hive connect 2
take-task
```

```python
# src/models/random_forest.py
import mlflow
from sklearn.ensemble import RandomForestClassifier

with mlflow.start_run(run_name="random-forest"):
    mlflow.log_param("model", "RandomForest")

    param_grid = {
        'n_estimators': [100, 200, 300],
        'max_depth': [10, 20, 30, None],
        'min_samples_split': [2, 5, 10],
        'min_samples_leaf': [1, 2, 4]
    }

    grid_search = GridSearchCV(
        RandomForestClassifier(random_state=42),
        param_grid,
        cv=5,
        scoring='roc_auc',
        n_jobs=-1,
        verbose=2
    )
    grid_search.fit(X_train, y_train)

    best_model = grid_search.best_estimator_
    mlflow.log_params(grid_search.best_params_)

    # Feature importance
    importances = best_model.feature_importances_
    mlflow.log_dict(dict(zip(feature_names, importances)), "feature_importance.json")

    # Evaluate
    y_pred = best_model.predict_proba(X_test)[:, 1]
    auc = roc_auc_score(y_test, y_pred)
    mlflow.log_metric("auc", auc)

    mlflow.sklearn.log_model(best_model, "model")
```

**Terminal 4 - Drone 3 (XGBoost with Optuna):**
```bash
hive connect 3
take-task
```

```python
# src/models/xgboost_optuna.py
import mlflow
import optuna
import xgboost as xgb

def objective(trial):
    params = {
        'max_depth': trial.suggest_int('max_depth', 3, 10),
        'learning_rate': trial.suggest_float('learning_rate', 0.01, 0.3),
        'n_estimators': trial.suggest_int('n_estimators', 100, 1000),
        'min_child_weight': trial.suggest_int('min_child_weight', 1, 10),
        'subsample': trial.suggest_float('subsample', 0.6, 1.0),
        'colsample_bytree': trial.suggest_float('colsample_bytree', 0.6, 1.0),
    }

    model = xgb.XGBClassifier(**params, random_state=42)
    model.fit(X_train, y_train)

    y_pred = model.predict_proba(X_test)[:, 1]
    auc = roc_auc_score(y_test, y_pred)

    return auc

with mlflow.start_run(run_name="xgboost-optuna"):
    study = optuna.create_study(direction='maximize')
    study.optimize(objective, n_trials=50)

    # Best params
    mlflow.log_params(study.best_params)
    mlflow.log_metric("auc", study.best_value)

    # Train final model
    best_model = xgb.XGBClassifier(**study.best_params, random_state=42)
    best_model.fit(X_train, y_train)

    mlflow.xgboost.log_model(best_model, "model")
```

**Terminal 5 - Drone 4 (Neural Network):**
```bash
hive connect 4
take-task
```

```python
# src/models/neural_network.py
import mlflow
import tensorflow as tf

with mlflow.start_run(run_name="neural-network"):
    model = tf.keras.Sequential([
        tf.keras.layers.Dense(64, activation='relu', input_shape=(n_features,)),
        tf.keras.layers.Dropout(0.3),
        tf.keras.layers.Dense(32, activation='relu'),
        tf.keras.layers.Dropout(0.3),
        tf.keras.layers.Dense(16, activation='relu'),
        tf.keras.layers.Dense(1, activation='sigmoid')
    ])

    model.compile(
        optimizer='adam',
        loss='binary_crossentropy',
        metrics=['AUC']
    )

    early_stopping = tf.keras.callbacks.EarlyStopping(
        monitor='val_auc',
        patience=10,
        restore_best_weights=True
    )

    history = model.fit(
        X_train, y_train,
        validation_split=0.2,
        epochs=100,
        batch_size=32,
        callbacks=[early_stopping],
        verbose=0
    )

    # Evaluate
    y_pred = model.predict(X_test)
    auc = roc_auc_score(y_test, y_pred)

    mlflow.log_metric("auc", auc)
    mlflow.tensorflow.log_model(model, "model")
```

### Step 3: Compare Results

Queen checks MLflow UI:
```bash
mlflow ui
# Browse to http://localhost:5000
# Compare all 4 experiments
```

Results:
```
Model                 AUC      Training Time
─────────────────────────────────────────────
Logistic Regression   0.82     2 min
Random Forest         0.86     8 min
XGBoost (Optuna)      0.89     12 min
Neural Network        0.88     10 min
```

Best model: **XGBoost with AUC 0.89**

## Common Tasks

### Feature Engineering

```bash
hive-assign drone-1 "Create time-based features" "Add day_of_week, hour, is_weekend from timestamp" "ML-105"
hive-assign drone-2 "Create aggregation features" "Add user stats: total_purchases, avg_amount, etc." "ML-106"
hive-assign drone-3 "Create text features" "TF-IDF on product descriptions" "ML-107"
```

### Model Evaluation

```bash
hive-assign drone-1 "Calculate business metrics" "Revenue impact, cost-benefit analysis" "ML-108"
hive-assign drone-2 "Fairness analysis" "Check for bias across demographics" "ML-109"
hive-assign drone-3 "Error analysis" "Analyze false positives and false negatives" "ML-110"
```

### Production Pipeline

```bash
hive-assign drone-1 "Create inference API" "FastAPI endpoint for real-time predictions" "ML-111"
hive-assign drone-2 "Add model monitoring" "Log predictions, track drift with Evidently" "ML-112"
hive-assign drone-3 "Setup batch predictions" "Airflow DAG for daily batch scoring" "ML-113"
```

## Best Practices

### 1. Track Everything

```python
with mlflow.start_run():
    mlflow.log_params({"param": value})
    mlflow.log_metrics({"metric": score})
    mlflow.log_artifact("plot.png")
    mlflow.log_model(model, "model")
```

### 2. Reproducibility

```python
# Set all random seeds
import random
import numpy as np
import tensorflow as tf

random.seed(42)
np.random.seed(42)
tf.random.set_seed(42)
```

### 3. Test Before task-done

```bash
pytest tests/ -v --cov=src --cov-report=html
# Require >80% coverage
```

## Troubleshooting

### CUDA Out of Memory

```python
# Reduce batch size
batch_size = 16  # Instead of 32

# Or use gradient accumulation
for i, batch in enumerate(dataloader):
    loss = model(batch)
    loss.backward()
    if (i + 1) % accumulation_steps == 0:
        optimizer.step()
        optimizer.zero_grad()
```

### MLflow Tracking Issues

```bash
# Check MLflow server
mlflow server --backend-store-uri sqlite:///mlflow.db --default-artifact-root ./mlruns

# Set tracking URI
export MLFLOW_TRACKING_URI=http://localhost:5000
```

## Timeline

**Sequential:**
- 4 models × 3 hours each = 12 hours

**Parallel (Hive):**
- 4 models in parallel = 3 hours

**Time saved: 75%**
