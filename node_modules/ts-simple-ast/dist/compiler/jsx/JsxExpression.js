"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var expression_1 = require("../expression");
var JsxExpression = /** @class */ (function (_super) {
    tslib_1.__extends(JsxExpression, _super);
    function JsxExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the dot dot dot token (...) or throws if it doesn't exist.
     */
    JsxExpression.prototype.getDotDotDotTokenOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getDotDotDotToken(), "Expected to find a dot dot dot token for the JSX expression.");
    };
    /**
     * Gets the dot dot dot token (...) or returns undefined if it doesn't exist.
     */
    JsxExpression.prototype.getDotDotDotToken = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.dotDotDotToken);
    };
    /**
     * Gets the expression or throws if it doesn't exist.
     */
    JsxExpression.prototype.getExpressionOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getExpression(), "Expected to find an expression for the JSX expression.");
    };
    /**
     * Gets the expression or returns undefined if it doesn't exist
     */
    JsxExpression.prototype.getExpression = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.expression);
    };
    return JsxExpression;
}(expression_1.Expression));
exports.JsxExpression = JsxExpression;
