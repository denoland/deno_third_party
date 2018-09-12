"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
function ModifierableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getModifiers = function () {
            var _this = this;
            return this.getCompilerModifiers().map(function (m) { return _this.getNodeFromCompilerNode(m); });
        };
        class_1.prototype.getFirstModifierByKindOrThrow = function (kind) {
            return errors.throwIfNullOrUndefined(this.getFirstModifierByKind(kind), "Expected a modifier of syntax kind: " + utils_1.getSyntaxKindName(kind));
        };
        class_1.prototype.getFirstModifierByKind = function (kind) {
            var e_1, _a;
            try {
                for (var _b = tslib_1.__values(this.getCompilerModifiers()), _c = _b.next(); !_c.done; _c = _b.next()) {
                    var modifier = _c.value;
                    if (modifier.kind === kind)
                        return this.getNodeFromCompilerNode(modifier);
                }
            }
            catch (e_1_1) { e_1 = { error: e_1_1 }; }
            finally {
                try {
                    if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
                }
                finally { if (e_1) throw e_1.error; }
            }
            return undefined;
        };
        class_1.prototype.hasModifier = function (textOrKind) {
            if (typeof textOrKind === "string")
                return this.getModifiers().some(function (m) { return m.getText() === textOrKind; });
            else
                return this.getCompilerModifiers().some(function (m) { return m.kind === textOrKind; });
        };
        class_1.prototype.toggleModifier = function (text, value) {
            if (value == null)
                value = !this.hasModifier(text);
            if (value)
                this.addModifier(text);
            else
                this.removeModifier(text);
            return this;
        };
        class_1.prototype.addModifier = function (text) {
            var modifiers = this.getModifiers();
            var existingModifier = utils_1.ArrayUtils.find(modifiers, function (m) { return m.getText() === text; });
            if (existingModifier != null)
                return existingModifier;
            // get insert position & index
            var _a = getInsertInfo(this), insertPos = _a.insertPos, insertIndex = _a.insertIndex;
            // insert setup
            var startPos;
            var newText;
            var isFirstModifier = modifiers.length === 0 || insertPos === modifiers[0].getStart();
            if (isFirstModifier) {
                newText = text + " ";
                startPos = insertPos;
            }
            else {
                newText = " " + text;
                startPos = insertPos + 1;
            }
            // insert
            manipulation_1.insertIntoParentTextRange({
                parent: modifiers.length === 0 ? this : modifiers[0].getParentSyntaxListOrThrow(),
                insertPos: insertPos,
                newText: newText
            });
            return utils_1.ArrayUtils.find(this.getModifiers(), function (m) { return m.getStart() === startPos; });
            function getInsertInfo(node) {
                var e_2, _a;
                var pos = getInitialInsertPos();
                var index = 0;
                try {
                    for (var _b = tslib_1.__values(getAddAfterModifierTexts(text)), _c = _b.next(); !_c.done; _c = _b.next()) {
                        var addAfterText = _c.value;
                        for (var i = 0; i < modifiers.length; i++) {
                            var modifier = modifiers[i];
                            if (modifier.getText() === addAfterText) {
                                if (pos < modifier.getEnd()) {
                                    pos = modifier.getEnd();
                                    index = i + 1;
                                }
                                break;
                            }
                        }
                    }
                }
                catch (e_2_1) { e_2 = { error: e_2_1 }; }
                finally {
                    try {
                        if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
                    }
                    finally { if (e_2) throw e_2.error; }
                }
                return { insertPos: pos, insertIndex: index };
                function getInitialInsertPos() {
                    var e_3, _a;
                    if (modifiers.length > 0)
                        return modifiers[0].getStart();
                    try {
                        for (var _b = tslib_1.__values(node.getChildrenIterator()), _c = _b.next(); !_c.done; _c = _b.next()) {
                            var child = _c.value;
                            // skip over any initial syntax lists (ex. decorators) or js docs
                            if (child.getKind() === typescript_1.SyntaxKind.SyntaxList || typescript_1.ts.isJSDocCommentContainingNode(child.compilerNode))
                                continue;
                            return child.getStart();
                        }
                    }
                    catch (e_3_1) { e_3 = { error: e_3_1 }; }
                    finally {
                        try {
                            if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
                        }
                        finally { if (e_3) throw e_3.error; }
                    }
                    return node.getStart();
                }
            }
        };
        class_1.prototype.removeModifier = function (text) {
            var modifiers = this.getModifiers();
            var modifier = utils_1.ArrayUtils.find(modifiers, function (m) { return m.getText() === text; });
            if (modifier == null)
                return false;
            manipulation_1.removeChildren({
                children: [modifiers.length === 1 ? modifier.getParentSyntaxListOrThrow() : modifier],
                removeFollowingSpaces: true
            });
            return true;
        };
        class_1.prototype.getCompilerModifiers = function () {
            return this.compilerNode.modifiers || [];
        };
        return class_1;
    }(Base));
}
exports.ModifierableNode = ModifierableNode;
/**
 * @returns The texts the specified text should appear after.
 */
function getAddAfterModifierTexts(text) {
    switch (text) {
        case "export":
            return [];
        case "default":
            return ["export"];
        case "declare":
            return ["export", "default"];
        case "abstract":
            return ["export", "default", "declare", "public", "private", "protected"];
        case "readonly":
            return ["export", "default", "declare", "public", "private", "protected", "abstract", "static"];
        case "public":
        case "protected":
        case "private":
            return [];
        case "static":
            return ["public", "protected", "private"];
        case "async":
            return ["export", "public", "protected", "private", "static", "abstract"];
        case "const":
            return [];
        /* istanbul ignore next */
        default:
            throw new errors.NotImplementedError("Not implemented modifier: " + text);
    }
}
