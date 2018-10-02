"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expressioned_1 = require("./expressioned");
var UnaryExpression_1 = require("./UnaryExpression");
exports.DeleteExpressionBase = expressioned_1.UnaryExpressionedNode(UnaryExpression_1.UnaryExpression);
var DeleteExpression = /** @class */ (function (_super) {
    tslib_1.__extends(DeleteExpression, _super);
    function DeleteExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return DeleteExpression;
}(exports.DeleteExpressionBase));
exports.DeleteExpression = DeleteExpression;
