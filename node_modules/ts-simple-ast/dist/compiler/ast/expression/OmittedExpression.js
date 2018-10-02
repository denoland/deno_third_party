"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
exports.OmittedExpressionBase = Expression_1.Expression;
var OmittedExpression = /** @class */ (function (_super) {
    tslib_1.__extends(OmittedExpression, _super);
    function OmittedExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return OmittedExpression;
}(exports.OmittedExpressionBase));
exports.OmittedExpression = OmittedExpression;
