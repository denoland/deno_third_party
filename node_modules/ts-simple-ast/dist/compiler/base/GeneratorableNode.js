"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function GeneratorableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.isGenerator = function () {
            return this.compilerNode.asteriskToken != null;
        };
        class_1.prototype.getAsteriskToken = function () {
            return this.getNodeFromCompilerNodeIfExists(this.compilerNode.asteriskToken);
        };
        class_1.prototype.getAsteriskTokenOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getAsteriskToken(), "Expected to find an asterisk token.");
        };
        class_1.prototype.setIsGenerator = function (value) {
            var asteriskToken = this.getAsteriskToken();
            var isSet = asteriskToken != null;
            if (isSet === value)
                return this;
            if (asteriskToken == null) {
                manipulation_1.insertIntoParentTextRange({
                    insertPos: getAsteriskInsertPos(this),
                    parent: this,
                    newText: "*"
                });
            }
            else {
                manipulation_1.removeChildrenWithFormatting({
                    children: [asteriskToken],
                    getSiblingFormatting: function () { return manipulation_1.FormattingKind.Space; }
                });
            }
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isGenerator != null)
                this.setIsGenerator(structure.isGenerator);
            return this;
        };
        return class_1;
    }(Base));
}
exports.GeneratorableNode = GeneratorableNode;
function getAsteriskInsertPos(node) {
    if (node.getKind() === typescript_1.SyntaxKind.FunctionDeclaration)
        return node.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.FunctionKeyword).getEnd();
    var namedNode = node;
    /* istanbul ignore if */
    if (namedNode.getName == null)
        throw new errors.NotImplementedError("Expected a name node for a non-function declaration.");
    return namedNode.getNameNode().getStart();
}
