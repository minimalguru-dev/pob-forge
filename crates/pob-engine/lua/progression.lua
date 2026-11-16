-- Read-only snapshots use PoB's CompareEntry; neither the live build nor Compare's selection is replaced.
return function(methods, ensureBuild, decodeCode)
	local array, null = __native.array, __native.null
	local function scalarCopy(input)
		local out = {}
		for k, v in pairs(input or {}) do
			if type(v) == "string" or type(v) == "boolean" or type(v) == "number" then out[k] = v end
		end
		return out
	end
	local function stats(entry)
		local o = entry:GetOutput() or {}
		local out = {}
		for _, key in ipairs({ "CombinedDPS", "TotalDPS", "FullDPS", "Speed", "Life", "EnergyShield", "TotalEHP",
			"PhysicalMaximumHitTaken", "FireMaximumHitTaken", "ColdMaximumHitTaken", "LightningMaximumHitTaken",
			"ChaosMaximumHitTaken", "FireResist", "ColdResist", "LightningResist", "ChaosResist",
			"FireResistMax", "ColdResistMax", "LightningResistMax", "Str", "Dex", "Int", "ReqStr", "ReqDex", "ReqInt",
			"ManaUnreserved", "ManaCostWarning", "LifeCostWarning", "SpellSuppressionChance" }) do
			local v = o[key]
			-- Maximum resistances are not in PoB's output table.
			local elem = key:match("^(%a+)ResistMax$")
			if elem then v = methods._resistMax(entry.calcsTab.mainEnv, elem) end
			if type(v) == "number" or type(v) == "boolean" then out[key] = v else out[key] = null end
		end
		return out
	end
	return function(p)
		ensureBuild()
		if methods.version().game ~= "poe1" then error("Progression supports Path of Exile 1 only", 0) end
		p = p or {}
		local xml = p.xml or (p.code and decodeCode(p.code)) or methods.save_build_xml().xml
		if type(xml) ~= "string" or #xml > 4 * 1024 * 1024 then error("PoB XML must be smaller than 4 MiB", 0) end
		local parsed, err = common.xml.ParseXML(xml)
		if err or not parsed or not parsed[1] or parsed[1].elem ~= "PathOfBuilding" then
			error("A valid Path of Exile 1 PoB build is required", 0)
		end
		local hasBuild = false
		for _, node in ipairs(parsed[1]) do
			if type(node) == "table" and node.elem == "Build" then hasBuild = true end
		end
		if not hasBuild then error("PoB XML has no Build section", 0) end
		local label = tostring(p.label or "Snapshot")
		if #label > 640 then error("Snapshot name is too long", 0) end
		local entry = new("CompareEntry"):CompareEntry(xml, label)
		if not entry or not entry.calcsTab or not entry.spec then error("PoB could not load this build", 0) end
		local spec = entry.spec
		local nodes, equipment, gems, errors = array({}), array({}), array({}), array({})
		for id, node in pairs(spec.nodes) do
			if node.alloc then nodes[#nodes + 1] = { id = id, name = node.dn or tostring(id), mastery = (spec.masterySelections or {})[id] or null } end
		end
		table.sort(nodes, function(a, b) return a.id < b.id end)
		for _, slot in ipairs(entry.itemsTab.orderedSlots) do
			local shown = slot.shown ~= false
			if type(slot.shown) == "function" then shown = slot.shown() end
			if shown and not slot.inactive then
				local item = slot.selItemId and entry.itemsTab.items[slot.selItemId]
				if item then equipment[#equipment + 1] = {
					slot = slot.slotName, name = item.name, raw = item:BuildRaw(),
					variant = item.variant or null, variant2 = item.variant2 or null,
				} end
			end
		end
		for gi, group in ipairs(entry.skillsTab.socketGroupList or {}) do
			for _, gem in ipairs(group.gemList or {}) do
				gems[#gems + 1] = { group = gi, name = gem.nameSpec or "Unknown", level = gem.level or 1,
					quality = gem.quality or 0, enabled = group.enabled ~= false and gem.enabled ~= false }
				if gem.errMsg and group.enabled ~= false and gem.enabled ~= false then errors[#errors + 1] = tostring(gem.errMsg) end
			end
		end
		local skill = entry.calcsTab.mainEnv and entry.calcsTab.mainEnv.player.mainSkill
		local effect = skill and skill.activeEffect and skill.activeEffect.grantedEffect
		local version = methods.version()
		local result = {
			xml = xml, label = label, game = "poe1", pobVersion = version.pobVersion, treeVersion = spec.treeVersion,
			level = entry.characterLevel, className = spec.curClassName, ascendancy = spec.curAscendClassName,
			mainSkill = effect and effect.name or null, nodes = nodes, equipment = equipment, gems = gems,
			gemErrors = errors, config = scalarCopy(entry.configTab.input), stats = stats(entry),
			loadout = { tree = spec.title or "Default", items = tostring(entry.itemsTab.activeItemSetId), skills = tostring(entry.skillsTab.activeSkillSetId) },
		}
		local boss = p.boss or "Pinnacle"
		if boss ~= "Boss" and boss ~= "Pinnacle" and boss ~= "Uber" then error("Unknown boss profile", 0) end
		local input = entry.configTab.input
		local overrides = { enemyIsBoss = boss, useFrenzyCharges = false, usePowerCharges = false,
			useEnduranceCharges = false, buffOnslaught = false, multiplierRampage = 0 }
		for _, option in ipairs(require("Modules.ConfigOptions")) do
			if option.var and option.var:find("Killed") then
				if option.type == "check" then overrides[option.var] = false else overrides[option.var] = 0 end
			end
		end
		for key, value in pairs(overrides) do input[key] = value end
		for i = 1, 5 do
			local slot = entry.itemsTab.slots["Flask " .. i]
			if slot then slot.active = false end
		end
		entry.configTab:BuildModList()
		entry:Rebuild()
		result.boss = { kind = boss, stats = stats(entry), overrides = overrides }
		return result
	end
end
