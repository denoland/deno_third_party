"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Node_1 = require("./Node");
var ComputedPropertyName = /** @class */ (function (_super) {
    tslib_1.__extends(ComputedPropertyName, _super);
    function ComputedPropertyName() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the expression.
     */
    ComputedPropertyName.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return ComputedPropertyName;
}(Node_1.Node));
exports.ComputedPropertyName = ComputedPropertyName;
