import pandas as pd
import numpy as np
from sklearn.model_selection import cross_val_score, GridSearchCV
from models4regressor import LinearRegressionModel, DecisionTreeRegressorModel, RandomForestRegressorModel, RidgeModel, LassoModel, ElasticNetModel, KNeighborsRegressorModel, GradientBoostingRegressorModel

data = pd.read_csv('xxx.csv')
X = data.iloc[:, :8].values
y = data['area'].values

def cross_val_evaluate(model, X, y, param_grid, n_folds=10):
    grid_search = GridSearchCV(model, param_grid, cv=n_folds, scoring='neg_mean_absolute_error')
    grid_search.fit(X, y)
    best_model = grid_search.best_estimator_
    mae_scores = -cross_val_score(best_model, X, y, cv=n_folds, scoring='neg_mean_absolute_error')
    mape = cross_val_score(best_model, X, y, scoring='neg_mean_absolute_percentage_error', cv=n_folds)
    rrse_scores = np.sqrt(-cross_val_score(best_model, X, y, cv=n_folds, scoring='neg_mean_squared_error')) / np.std(y)
    r_scores = cross_val_score(best_model, X, y, cv=n_folds, scoring='r2')
    rmse_scores = np.sqrt(-cross_val_score(best_model, X, y, cv=n_folds, scoring='neg_mean_squared_error'))
    return mae_scores.mean(), rrse_scores.mean(), r_scores.mean(), mape.mean(), rmse_scores.mean(), grid_search.best_params_

# Linear Regression
lr = LinearRegressionModel()
lr_param_grid = {}
lr_mae, lr_rrse, lr_r, lr_mape, lr_rmse, lr_best_params = cross_val_evaluate(lr.model, X, y, lr_param_grid)

# Decision Tree
dt = DecisionTreeRegressorModel()
dt_param_grid = {'max_depth': [2, 4, 6, 8, 10]}
dt_mae, dt_rrse, dt_r, dt_mape, dt_rmse, dt_best_params = cross_val_evaluate(dt.model, X, y, dt_param_grid)

# Random Forest
rf = RandomForestRegressorModel()
rf_param_grid = {'n_estimators': [50, 100, 200], 'max_depth': [2, 4, 6, 8, 10]}
rf_mae, rf_rrse, rf_r, rf_mape, rf_rmse, rf_best_params = cross_val_evaluate(rf.model, X, y, rf_param_grid)

# Ridge Regression
ridge = RidgeModel()
ridge_param_grid = {'alpha': [0.1, 1.0, 10.0]}
ridge_mae, ridge_rrse, ridge_r, ridge_mape, ridge_rmse, ridge_best_params = cross_val_evaluate(ridge.model, X, y, ridge_param_grid)

# Lasso Regression
lasso = LassoModel()
lasso_param_grid = {'alpha': [0.1, 1.0, 10.0]}
lasso_mae, lasso_rrse, lasso_r, lasso_mape, lasso_rmse, lasso_best_params = cross_val_evaluate(lasso.model, X, y, lasso_param_grid)

# Elastic Net Regression
elastic_net = ElasticNetModel()
elastic_net_param_grid = {'alpha': [0.1, 1.0, 10.0], 'l1_ratio': [0.2, 0.5, 0.8]}
elastic_net_mae, elastic_net_rrse, elastic_net_r, elastic_net_mape, elastic_net_rmse, elastic_net_best_params = cross_val_evaluate(elastic_net.model, X, y, elastic_net_param_grid)

# KNN Regression
knn = KNeighborsRegressorModel()
knn_param_grid = {'n_neighbors': [3, 5, 7, 9]}
knn_mae, knn_rrse, knn_r, knn_mape, knn_rmse, knn_best_params = cross_val_evaluate(knn.model, X, y, knn_param_grid)

# Gradient Boosting Regression
gbr = GradientBoostingRegressorModel()
gbr_param_grid = {'n_estimators': [50, 100, 200], 'learning_rate': [0.01, 0.1, 0.5], 'max_depth': [2, 4, 6]}
gbr_mae, gbr_rrse, gbr_r, gbr_mape, gbr_rmse, gbr_best_params = cross_val_evaluate(gbr.model, X, y, gbr_param_grid)

print(
    f"Linear Regression: MAE={lr_mae}, RRSE={lr_rrse}, R_square={lr_r}, MAPE={lr_mape}, RMSE={lr_rmse}, Best Params={lr_best_params}"
)
print(
    f"Decision Tree: MAE={dt_mae}, RRSE={dt_rrse}, R_square={dt_r}, MAPE={dt_mape}, RMSE={dt_rmse}, Best Params={dt_best_params}"
)
print(
    f"Random Forest: MAE={rf_mae}, RRSE={rf_rrse}, R_square={rf_r}, MAPE={rf_mape}, RMSE={rf_rmse}, Best Params={rf_best_params}"
)
print(
    f"Ridge Regression: MAE={ridge_mae}, RRSE={ridge_rrse}, R_square={ridge_r}, MAPE={ridge_mape}, RMSE={ridge_rmse}, Best Params={ridge_best_params}"
)
print(
    f"Lasso Regression: MAE={lasso_mae}, RRSE={lasso_rrse}, R_square={lasso_r}, MAPE={lasso_mape}, RMSE={lasso_rmse}, Best Params={lasso_best_params}"
)
print(
    f"Elastic Net Regression: MAE={elastic_net_mae}, RRSE={elastic_net_rrse}, R_square={elastic_net_r}, MAPE={elastic_net_mape}, RMSE={elastic_net_rmse}, Best Params={elastic_net_best_params}"
)
print(
    f"KNN Regression: MAE={knn_mae}, RRSE={knn_rrse}, R_square={knn_r}, MAPE={knn_mape}, RMSE={knn_rmse}, Best Params={knn_best_params}"
)
print(
    f"Gradient Boosting Regression: MAE={gbr_mae}, RRSE={gbr_rrse}, R_square={gbr_r}, MAPE={gbr_mape}, RMSE={gbr_rmse}, Best Params={gbr_best_params}"
)