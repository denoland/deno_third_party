"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../../errors");
var utils_1 = require("../../../utils");
function InitializerGetExpressionableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.hasInitializer = function () {
            return this.compilerNode.initializer != null;
        };
        class_1.prototype.getInitializerIfKindOrThrow = function (kind) {
            return errors.throwIfNullOrUndefined(this.getInitializerIfKind(kind), "Expected to find an initiailizer of kind '" + utils_1.getSyntaxKindName(kind) + "'.");
        };
        class_1.prototype.getInitializerIfKind = function (kind) {
            var initiailizer = this.getInitializer();
            if (initiailizer != null && initiailizer.getKind() !== kind)
                return undefined;
            return initiailizer;
        };
        class_1.prototype.getInitializerOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getInitializer(), "Expected to find an initializer.");
        };
        class_1.prototype.getInitializer = function () {
            return this.getNodeFromCompilerNodeIfExists(this.compilerNode.initializer);
        };
        return class_1;
    }(Base));
}
exports.InitializerGetExpressionableNode = InitializerGetExpressionableNode;
