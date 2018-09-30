"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var ElementAccessExpression_1 = require("./ElementAccessExpression");
var expressioned_1 = require("./expressioned");
exports.SuperElementAccessExpressionBase = expressioned_1.SuperExpressionedNode(ElementAccessExpression_1.ElementAccessExpression);
var SuperElementAccessExpression = /** @class */ (function (_super) {
    tslib_1.__extends(SuperElementAccessExpression, _super);
    function SuperElementAccessExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return SuperElementAccessExpression;
}(exports.SuperElementAccessExpressionBase));
exports.SuperElementAccessExpression = SuperElementAccessExpression;
