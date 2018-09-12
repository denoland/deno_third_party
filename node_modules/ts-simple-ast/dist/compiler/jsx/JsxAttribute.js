"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var common_1 = require("../common");
exports.JsxAttributeBase = base_1.NamedNode(common_1.Node);
var JsxAttribute = /** @class */ (function (_super) {
    tslib_1.__extends(JsxAttribute, _super);
    function JsxAttribute() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the JSX attribute's initializer or throws if it doesn't exist.
     */
    JsxAttribute.prototype.getInitializerOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getInitializer(), "Expected to find an initializer for the JSX attribute '" + this.getName() + "'");
    };
    /**
     * Gets the JSX attribute's initializer or returns undefined if it doesn't exist.
     */
    JsxAttribute.prototype.getInitializer = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.initializer);
    };
    /**
     * Removes the JSX attribute.
     */
    JsxAttribute.prototype.remove = function () {
        manipulation_1.removeChildren({
            children: [this],
            removePrecedingNewLines: true,
            removePrecedingSpaces: true
        });
    };
    return JsxAttribute;
}(exports.JsxAttributeBase));
exports.JsxAttribute = JsxAttribute;
