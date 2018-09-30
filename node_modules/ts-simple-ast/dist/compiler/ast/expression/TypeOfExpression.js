"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expressioned_1 = require("./expressioned");
var UnaryExpression_1 = require("./UnaryExpression");
exports.TypeOfExpressionBase = expressioned_1.UnaryExpressionedNode(UnaryExpression_1.UnaryExpression);
var TypeOfExpression = /** @class */ (function (_super) {
    tslib_1.__extends(TypeOfExpression, _super);
    function TypeOfExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return TypeOfExpression;
}(exports.TypeOfExpressionBase));
exports.TypeOfExpression = TypeOfExpression;
