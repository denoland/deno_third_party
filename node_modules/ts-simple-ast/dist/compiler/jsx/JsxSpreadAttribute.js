"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var common_1 = require("../common");
var JsxSpreadAttribute = /** @class */ (function (_super) {
    tslib_1.__extends(JsxSpreadAttribute, _super);
    function JsxSpreadAttribute() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the JSX spread attribute's expression.
     */
    JsxSpreadAttribute.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    /**
     * Removes the JSX spread attribute.
     */
    JsxSpreadAttribute.prototype.remove = function () {
        manipulation_1.removeChildren({
            children: [this],
            removePrecedingNewLines: true,
            removePrecedingSpaces: true
        });
    };
    return JsxSpreadAttribute;
}(common_1.Node));
exports.JsxSpreadAttribute = JsxSpreadAttribute;
