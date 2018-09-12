"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
function JsxTagNamedNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        /**
         * Gets the tag name of the JSX element.
         */
        class_1.prototype.getTagName = function () {
            return this.getNodeFromCompilerNode(this.compilerNode.tagName);
        };
        return class_1;
    }(Base));
}
exports.JsxTagNamedNode = JsxTagNamedNode;
