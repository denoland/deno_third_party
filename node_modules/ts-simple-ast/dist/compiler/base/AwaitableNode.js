"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function AwaitableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.isAwaited = function () {
            return this.compilerNode.awaitModifier != null;
        };
        class_1.prototype.getAwaitKeyword = function () {
            var awaitModifier = this.compilerNode.awaitModifier;
            return this.getNodeFromCompilerNodeIfExists(awaitModifier);
        };
        class_1.prototype.getAwaitKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getAwaitKeyword(), "Expected to find an await token.");
        };
        class_1.prototype.setIsAwaited = function (value) {
            var awaitModifier = this.getAwaitKeyword();
            var isSet = awaitModifier != null;
            if (isSet === value)
                return this;
            if (awaitModifier == null) {
                manipulation_1.insertIntoParentTextRange({
                    insertPos: getAwaitInsertPos(this),
                    parent: this,
                    newText: " await"
                });
            }
            else {
                manipulation_1.removeChildren({
                    children: [awaitModifier],
                    removePrecedingSpaces: true
                });
            }
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isAwaited != null)
                this.setIsAwaited(structure.isAwaited);
            return this;
        };
        return class_1;
    }(Base));
}
exports.AwaitableNode = AwaitableNode;
function getAwaitInsertPos(node) {
    if (node.getKind() === typescript_1.SyntaxKind.ForOfStatement)
        return node.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.ForKeyword).getEnd();
    throw new errors.NotImplementedError("Expected a for of statement node.");
}
